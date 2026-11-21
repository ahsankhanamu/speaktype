use crate::commands::AppState;
use crate::settings::Settings;
use serde_json::json;
use speaktype_session::types::{SessionDetail, Target};
use tauri::Manager;

/// Generate a document from a session.
///
/// Kept compiled (spec §10) for the later cleanup-modes feature; the Scribe UI
/// entry points were removed in Phase 0. No LLM configured → returns the plain
/// session render. An OpenAI-compatible endpoint (base URL + key + model in
/// Settings) is POSTed the template prompt + the session's rendered captures.
/// When "use local llama.cpp" is enabled the local `llama-server` is started
/// (if needed) and used instead of the configured remote base URL.
pub async fn generate_session_document(
    app: &tauri::AppHandle,
    session_id: i64,
) -> Result<String, String> {
    let (mut base_url, mut api_key, model, use_local, local_model, local_ctx) = {
        let state = app.state::<AppState>();
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        (
            settings.scribe_llm_base_url.clone().trim().to_string(),
            settings.scribe_llm_api_key.clone(),
            settings.scribe_llm_model.clone().trim().to_string(),
            settings.scribe_llm_use_local,
            settings.scribe_llm_local_model.clone(),
            settings.scribe_llm_local_ctx,
        )
    };

    let store = app.state::<std::sync::Arc<speaktype_session::Store>>();
    let rendered = crate::session_cmds::render_prompt(
        &crate::session_cmds::sessions_root(),
        &store,
        session_id,
        Target::Markdown,
    )
    .map_err(|e| e.to_string())?;
    let raw = rendered.text;

    // Resolve the template: the session's stored one, re-inferred from the
    // captured content when it is "auto".
    let detail = store.detail(session_id).map_err(|e| e.to_string())?;
    let template = if detail.session.template == "auto" {
        crate::templates::infer_template(&collective_session_text(&detail)).to_string()
    } else {
        detail.session.template.clone()
    };

    if use_local {
        match crate::llm_server::local_base_url() {
            Some(url) => base_url = url,
            None => {
                if local_model.trim().is_empty() {
                    return Err("Local LLM: no GGUF model path set in Scribe settings".to_string());
                }
                crate::logging::log_message("[llm] Starting local llama-server for generation");
                let info = crate::llm_server::start_llm_server(app, &local_model, local_ctx)?;
                if !crate::llm_server::wait_for_llm_server(info.port).await {
                    crate::llm_server::stop_llm_server();
                    return Err("Local LLM: llama-server failed to start in time".to_string());
                }
                base_url = crate::llm_server::local_base_url()
                    .ok_or("Local LLM: server port unavailable")?;
            }
        }
        api_key = String::new();
    }

    if base_url.is_empty() {
        crate::logging::log_message(
            "[llm] No endpoint configured — returning plain session render",
        );
        return Ok(raw);
    }

    if api_key.is_empty() && !use_local {
        return Err("LLM: API key is not set".to_string());
    }

    let url = base_url.trim_end_matches('/').to_string() + "/chat/completions";
    let model = if model.is_empty() {
        "gpt-4o-mini".to_string()
    } else {
        model
    };

    let system = crate::templates::template_prompt(&template).to_string();
    let user = format!(
        "Here are the raw captures from a working session:\n\n---\n{}\n---\n\nTask: {}",
        raw, system
    );

    let client = reqwest::Client::new();
    let mut req = client
        .post(&url)
        .json(&json!({
            "model": model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user },
            ],
            "temperature": 0.3,
            "max_tokens": 4096,
        }))
        .timeout(std::time::Duration::from_secs(300));
    if !api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", api_key));
    }

    let resp = req
        .send()
        .await
        .map_err(|e| format!("LLM request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("LLM error {}: {}", status, body));
    }

    let json_resp: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("LLM response parse failed: {}", e))?;
    let content = json_resp
        .pointer("/choices/0/message/content")
        .and_then(|c| c.as_str())
        .map(|s| s.trim().to_string())
        .ok_or_else(|| "LLM returned no message content".to_string())?;

    // LLMs routinely summarize screenshots away, so re-attach the actual
    // images — with absolute bundle paths — to the end of the returned
    // markdown, where previews and exports always show them.
    let absolute = crate::session_cmds::render_prompt(
        &crate::session_cmds::sessions_root(),
        &store,
        session_id,
        Target::CodingAgent,
    )
    .map_err(|e| e.to_string())?;
    let screenshots: String = absolute
        .images
        .iter()
        .map(|path| format!("![screenshot]({path})\n"))
        .collect();
    if !screenshots.trim().is_empty() {
        return Ok(format!("{}\n\n## Screenshots\n\n{}", content, screenshots));
    }
    Ok(content)
}

/// Concatenate every textual capture in a session (dictation, clipboard copies,
/// and screenshot OCR text) so template inference works on the real content.
fn collective_session_text(detail: &SessionDetail) -> String {
    let mut parts: Vec<String> = Vec::new();
    for item in &detail.items {
        if let Some(text) = item.text.as_deref().filter(|t| !t.trim().is_empty()) {
            parts.push(text.to_string());
        }
        if let Some(ocr) = item
            .payload
            .get("ocr")
            .and_then(|o| o.as_str())
            .filter(|t| !t.trim().is_empty())
        {
            parts.push(ocr.to_string());
        }
    }
    parts.join("\n")
}

/// Validate the LLM settings before showtime — cheap and useful for a Settings
/// UI to surface configuration problems immediately.
pub fn validate_llm_settings(settings: &Settings) -> Result<(), String> {
    if settings.scribe_llm_use_local {
        let model = settings.scribe_llm_local_model.trim();
        if model.is_empty() {
            return Err("Local LLM: no GGUF model path set".to_string());
        }
        if !std::path::Path::new(model).exists() {
            return Err(format!("Local LLM: GGUF model not found at {}", model));
        }
        return Ok(());
    }
    let base = settings.scribe_llm_base_url.trim();
    let key = settings.scribe_llm_api_key.trim();
    if base.is_empty() {
        return Ok(()); // no LLM → fall back to plain export (fine)
    }
    if key.is_empty() {
        return Err("API key is empty while a base URL is set".to_string());
    }
    if !(base.starts_with("http://") || base.starts_with("https://")) {
        return Err("Base URL must start with http:// or https://".to_string());
    }
    Ok(())
}
