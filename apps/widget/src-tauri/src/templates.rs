/// Persona templates for the LLM assembly step (Phase 2). For the MVP the
/// session renders to a plain chronological markdown document; these prompt
/// strings define how the assembled raw captures get restructured.
pub const TEMPLATES: &[(&str, &str)] = &[
    (
        "general",
        "You are a research assistant. The user captured a working session with \
         voice dictation, screenshots, and clipboard copies. Assemble the most \
         useful parts into a well-structured document with clear sections, key \
         findings, and a short summary.",
    ),
    (
        "thesis",
        "You are an academic writing assistant. The user is working on a thesis. \
         Turn their session captures into a structured thesis draft with \
         introduction, methods, findings, and references.",
    ),
    (
        "qa_test_plan",
        "You are a QA documentation assistant. Turn the session captures into a \
         structured test plan or bug report with clear reproduction steps, \
         expected vs actual behavior, and affected components.",
    ),
    (
        "blog",
        "You are a blog-writing assistant. Turn the user's session captures into \
         a well-structured blog post with an introduction, body sections, and a \
         conclusion.",
    ),
    (
        "meeting_notes",
        "You are a meeting-notes assistant. Turn the session captures into \
         structured meeting notes with decisions, action items, and key \
         discussion points.",
    ),
];

pub fn template_prompt(template: &str) -> &'static str {
    TEMPLATES
        .iter()
        .find(|(key, _)| *key == template)
        .map(|(_, prompt)| *prompt)
        .unwrap_or(TEMPLATES[0].1)
}

/// Lightweight, keyword-based template detection so the app can pick how to
/// assemble a document without asking the user. Used when the session is created
/// with "auto" and again at generation time on the real captured content.
pub fn infer_template(text: &str) -> &'static str {
    let t = text.to_lowercase();
    if contains_any(
        &t,
        &[
            "thesis",
            "dissertation",
            "methodology",
            "literature review",
            "abstract",
            "research question",
            "defense",
            "supervisor",
            "citation",
        ],
    ) {
        "thesis"
    } else if contains_any(
        &t,
        &[
            "meeting",
            "standup",
            "agenda",
            "action item",
            "action items",
            "decision",
            "attendee",
            "follow-up",
            "kickoff",
            "retro",
        ],
    ) {
        "meeting_notes"
    } else if contains_any(
        &t,
        &[
            "bug",
            "reproduc",
            "test plan",
            "test case",
            "regression",
            "qa",
            "staging",
            "error log",
            "crash report",
            "investigate",
            "expected vs actual",
        ],
    ) {
        "qa_test_plan"
    } else if contains_any(
        &t,
        &[
            "blog",
            "post",
            "article",
            "audience",
            "publish",
            "newsletter",
        ],
    ) {
        "blog"
    } else {
        "general"
    }
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| text.contains(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_session_type_from_name() {
        assert_eq!(
            infer_template("Thesis research — methodology review"),
            "thesis"
        );
        assert_eq!(infer_template("Standup and action items"), "meeting_notes");
        assert_eq!(infer_template("Regression bug on checkout"), "qa_test_plan");
        assert_eq!(infer_template("Blog post draft"), "blog");
        assert_eq!(infer_template("Catch-up session"), "general");
    }

    #[test]
    fn infers_from_captured_content_over_name() {
        let content = "discussed the kickoff agenda for the new sprint and assigned action items";
        assert_eq!(infer_template(content), "meeting_notes");
        let content = "reproduced the crash on staging, writing a test case";
        assert_eq!(infer_template(content), "qa_test_plan");
    }
}
