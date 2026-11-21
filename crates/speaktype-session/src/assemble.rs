use crate::types::{ItemKind, SessionDetail, Target};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Rendered prompt: deterministic text for a target, plus the bundle image
/// paths it references (in order) and their measured sizes.
#[derive(Serialize, Deserialize, specta::Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RenderedPrompt {
    pub text: String,
    /// Absolute bundle paths, in prompt order.
    pub images: Vec<String>,
    pub image_count: u32,
    pub total_image_bytes: u64,
    pub warning: Option<String>,
}

pub fn assemble(
    detail: &SessionDetail,
    target: Target,
    preface: &str,
    bundle_dir: &Path,
) -> RenderedPrompt {
    let mut segments: Vec<String> = Vec::new();
    let preface = preface.trim();
    if !preface.is_empty() {
        segments.push(preface.to_string());
    }

    let mut images: Vec<std::path::PathBuf> = Vec::new();
    let mut mark_n: u32 = 0;
    let mut screenshot_n: u32 = 0;
    let mut total_image_bytes = 0u64;
    let mut warnings: Vec<String> = Vec::new();

    for item in &detail.items {
        match item.kind {
            ItemKind::Speech | ItemKind::Note => {
                push_text(&mut segments, item.text.as_deref());
            }
            ItemKind::Clip => {
                if let Some(text) = item
                    .text
                    .as_deref()
                    .map(str::trim)
                    .filter(|t| !t.is_empty())
                {
                    let fence = fence_for(text);
                    segments.push(format!("{fence}text\n{text}\n{fence}"));
                }
            }
            ItemKind::Mark | ItemKind::Screenshot => {
                let (label, n) = if item.kind == ItemKind::Mark {
                    mark_n += 1;
                    ("Mark", mark_n)
                } else {
                    screenshot_n += 1;
                    ("Screenshot", screenshot_n)
                };
                let kind = if item.kind == ItemKind::Mark {
                    "mark"
                } else {
                    "screenshot"
                };
                let file = format!("images/{n:02}-{kind}.png");
                let path = bundle_dir.join(&file);
                let k = images.len() + 1;
                match std::fs::metadata(&path) {
                    Ok(md) => total_image_bytes += md.len(),
                    Err(_) => warnings.push(format!("Image {k} is missing.")),
                }
                images.push(path);
                match target {
                    Target::CodingAgent => {
                        let path = images.last().expect("just pushed");
                        segments.push(format!("![{label} {n}]({})", path.display()));
                    }
                    Target::Chat => segments.push(format!("[Image {k}]")),
                    Target::Markdown => segments.push(format!("![{label} {n}]({file})")),
                }
            }
        }
    }

    if images.len() > 20 {
        warnings.insert(0, "More than 20 images. Some apps may reject this.".into());
    }

    RenderedPrompt {
        text: segments.join("\n\n"),
        images: images.iter().map(|p| p.display().to_string()).collect(),
        image_count: images.len() as u32,
        total_image_bytes,
        warning: (!warnings.is_empty()).then(|| warnings.join("\n")),
    }
}

fn push_text(segments: &mut Vec<String>, text: Option<&str>) {
    if let Some(text) = text.map(str::trim).filter(|t| !t.is_empty()) {
        segments.push(text.to_string());
    }
}

/// One backtick longer than the longest run inside the clip, minimum 3.
fn fence_for(text: &str) -> String {
    let longest = text
        .chars()
        .fold((0usize, 0usize), |(best, run), c| {
            if c == '`' {
                (best.max(run + 1), run + 1)
            } else {
                (best, 0)
            }
        })
        .0;
    "`".repeat((longest + 1).max(3))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;
    use crate::types::{Item, NewItem, Session};
    use serde_json::json;

    fn session() -> Session {
        Session {
            id: 1,
            name: "S".into(),
            started_at_ms: 0,
            ended_at_ms: None,
            template: "auto".into(),
            target: Target::CodingAgent,
            preface: None,
            bundle_path: None,
        }
    }

    fn item(kind: ItemKind, ord: f64, text: Option<&str>, image_path: Option<&str>) -> Item {
        Item {
            id: ord as i64,
            session_id: 1,
            kind,
            ord,
            t_start_ms: 0,
            t_end_ms: None,
            text: text.map(str::to_string),
            caption: None,
            image_path: image_path.map(str::to_string),
            source_app: None,
            payload: json!({}),
        }
    }

    fn detail(items: Vec<Item>) -> SessionDetail {
        SessionDetail {
            session: session(),
            items,
        }
    }

    fn bundle() -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "speaktype_assemble_test_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        std::fs::create_dir_all(dir.join("images")).expect("bundle dir");
        dir
    }

    #[test]
    fn clip_containing_backticks_uses_longer_fence() {
        let d = detail(vec![item(
            ItemKind::Clip,
            1.0,
            Some("fenced ``` inside"),
            None,
        )]);
        let rendered = assemble(&d, Target::CodingAgent, "", Path::new("/bundle"));
        assert_eq!(rendered.text, "````text\nfenced ``` inside\n````");

        let d = detail(vec![item(
            ItemKind::Clip,
            1.0,
            Some("four ```` and more"),
            None,
        )]);
        let rendered = assemble(&d, Target::CodingAgent, "", Path::new("/bundle"));
        assert_eq!(rendered.text, "`````text\nfour ```` and more\n`````");
    }

    #[test]
    fn deleted_items_are_excluded() {
        let store = Store::open_in_memory().expect("store");
        let s = store.create_session("S", "auto", 0).expect("session");
        let speech = |t: &str| NewItem {
            kind: ItemKind::Speech,
            t_start_ms: 0,
            t_end_ms: None,
            text: Some(t.into()),
            image_path: None,
            source_app: None,
            payload: json!({}),
        };
        store.add_item(s.id, speech("kept line")).expect("add");
        let gone = store.add_item(s.id, speech("deleted line")).expect("add");
        store.delete_items(&[gone.id]).expect("delete");

        let d = store.detail(s.id).expect("detail");
        let rendered = assemble(&d, Target::CodingAgent, "", Path::new("/bundle"));
        assert!(rendered.text.contains("kept line"));
        assert!(!rendered.text.contains("deleted line"));
    }

    #[test]
    fn twenty_one_images_sets_warning() {
        let dir = bundle();
        let items: Vec<Item> = (1..=21)
            .map(|i| item(ItemKind::Mark, f64::from(i), None, Some("captures/x.png")))
            .collect();
        for i in 1..=21 {
            let file = dir.join(format!("images/{i:02}-mark.png"));
            std::fs::write(&file, b"png").expect("write image");
        }
        let d = detail(items);
        let rendered = assemble(&d, Target::CodingAgent, "", &dir);
        assert_eq!(rendered.image_count, 21);
        assert_eq!(
            rendered.warning.as_deref(),
            Some("More than 20 images. Some apps may reject this.")
        );
    }

    #[test]
    fn same_input_same_output() {
        let dir = bundle();
        let d = detail(vec![
            item(ItemKind::Speech, 1.0, Some("one"), None),
            item(ItemKind::Mark, 2.0, None, Some("captures/m.png")),
            item(ItemKind::Clip, 3.0, Some("two"), None),
        ]);
        std::fs::write(dir.join("images/01-mark.png"), b"png").expect("image");
        let first = assemble(&d, Target::Markdown, "P", &dir);
        let second = assemble(&d, Target::Markdown, "P", &dir);
        assert_eq!(first, second);
    }

    #[test]
    fn missing_image_counts_zero_and_warns() {
        let dir = bundle();
        let d = detail(vec![item(
            ItemKind::Mark,
            1.0,
            None,
            Some("captures/gone.png"),
        )]);
        let rendered = assemble(&d, Target::CodingAgent, "", &dir);
        assert_eq!(rendered.total_image_bytes, 0);
        assert_eq!(rendered.warning.as_deref(), Some("Image 1 is missing."));
    }

    #[test]
    fn empty_preface_has_no_leading_blank_line() {
        let dir = bundle();
        let d = detail(vec![item(ItemKind::Speech, 1.0, Some("hello"), None)]);
        let rendered = assemble(&d, Target::Chat, "", &dir);
        assert_eq!(rendered.text, "hello");
        let rendered = assemble(&d, Target::Chat, "  ", &dir);
        assert_eq!(rendered.text, "hello", "whitespace-only preface is empty");
    }
}
