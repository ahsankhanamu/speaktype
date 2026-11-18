//! Golden tests: the assembled prompt for a fixture session is byte-for-byte
//! equal to the committed golden files. Coding-agent paths are absolute, so
//! the test substitutes the per-run bundle dir with `{{bundle}}` before
//! comparing.

use speaktype_session::assemble::assemble;
use speaktype_session::types::{Item, ItemKind, Session, SessionDetail, Target};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

const PREFACE: &str =
    "You are helping me debug a failing checkout flow. Here are my notes and captures.";

fn temp_bundle() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "speaktype_golden_test_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(dir.join("images")).expect("bundle dir");
    dir
}

fn item(id: i64, kind: ItemKind, ord: f64, text: Option<&str>, image: Option<&str>) -> Item {
    Item {
        id,
        session_id: 1,
        kind,
        ord,
        t_start_ms: 0,
        t_end_ms: None,
        text: text.map(str::to_string),
        caption: None,
        image_path: image.map(str::to_string),
        source_app: None,
        payload: serde_json::json!({}),
    }
}

/// 2 speech, 2 marks, 1 screenshot, 1 clip — in mixed `ord`.
fn fixture() -> SessionDetail {
    SessionDetail {
        session: Session {
            id: 1,
            name: "Checkout bug".into(),
            started_at_ms: 0,
            ended_at_ms: None,
            template: "auto".into(),
            target: Target::CodingAgent,
            preface: None,
            bundle_path: None,
        },
        items: vec![
            item(
                1,
                ItemKind::Speech,
                1.0,
                Some("The checkout page rejects a valid coupon."),
                None,
            ),
            item(2, ItemKind::Mark, 2.0, None, Some("captures/m1.png")),
            item(
                3,
                ItemKind::Speech,
                3.0,
                Some("Note the error banner says the code expired."),
                None,
            ),
            item(
                4,
                ItemKind::Clip,
                4.0,
                Some("curl -i https://api.example.com/checkout"),
                None,
            ),
            item(5, ItemKind::Mark, 5.0, None, Some("captures/m2.png")),
            item(6, ItemKind::Screenshot, 6.0, None, Some("captures/s1.png")),
        ],
    }
}

fn golden(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join(name);
    std::fs::read_to_string(path).expect("golden file")
}

fn render(target: Target, bundle_dir: &std::path::Path) -> speaktype_session::RenderedPrompt {
    let rendered = assemble(&fixture(), target.clone(), PREFACE, bundle_dir);
    // The dummy images must exist so no "missing" warning fires.
    for (i, path) in rendered.images.iter().enumerate() {
        let _ = std::fs::write(path, format!("img{i}"));
    }
    assemble(&fixture(), target, PREFACE, bundle_dir)
}

#[test]
fn golden_coding_agent() {
    let bundle_dir = temp_bundle();
    let rendered = render(Target::CodingAgent, &bundle_dir);
    assert_eq!(rendered.image_count, 3);
    assert_eq!(rendered.warning, None);
    let total: u64 = ["img0", "img1", "img2"]
        .iter()
        .map(|s| s.len() as u64)
        .sum();
    assert_eq!(rendered.total_image_bytes, total);
    let actual = rendered
        .text
        .replace(bundle_dir.to_str().unwrap(), "{{bundle}}");
    assert_eq!(actual, golden("coding_agent.md"));
}

#[test]
fn golden_chat() {
    let bundle_dir = temp_bundle();
    let rendered = render(Target::Chat, &bundle_dir);
    assert_eq!(rendered.image_count, 3);
    assert_eq!(rendered.warning, None);
    assert_eq!(rendered.text, golden("chat.md"));
}

#[test]
fn golden_markdown() {
    let bundle_dir = temp_bundle();
    let rendered = render(Target::Markdown, &bundle_dir);
    assert_eq!(rendered.image_count, 3);
    assert_eq!(rendered.warning, None);
    assert_eq!(rendered.text, golden("markdown.md"));
}
