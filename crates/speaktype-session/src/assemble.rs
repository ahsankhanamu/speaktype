use crate::types::{ItemKind, SessionDetail, Target};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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

/// One piece of a session, in order: what every rendering is built from, so
/// the plain text and the HTML of a copy can never disagree.
#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    Text(String),
    Clip(String),
    Image {
        /// "Mark 1", "Screenshot 2".
        label: String,
        /// The bundle file (`<bundle>/images/01-mark.png`).
        bundle_path: PathBuf,
        /// The same file relative to the bundle (`images/01-mark.png`).
        file: String,
        /// The original capture, read when the bundle isn't written yet.
        capture: Option<PathBuf>,
    },
}

impl Block {
    /// Where an image's pixels are right now: the bundle file once written,
    /// else the capture.
    fn readable_image(&self) -> Option<&Path> {
        let Block::Image {
            bundle_path,
            capture,
            ..
        } = self
        else {
            return None;
        };
        if bundle_path.is_file() {
            Some(bundle_path)
        } else {
            capture.as_deref().filter(|p| p.is_file())
        }
    }
}

/// The session's live items as blocks, in order.
pub fn blocks(detail: &SessionDetail, bundle_dir: &Path) -> Vec<Block> {
    let (mut marks, mut screenshots) = (0u32, 0u32);
    let mut out = Vec::new();
    for item in &detail.items {
        let text = item
            .text
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty());
        match item.kind {
            ItemKind::Speech | ItemKind::Note => {
                if let Some(text) = text {
                    out.push(Block::Text(text.to_string()));
                }
            }
            ItemKind::Clip => {
                if let Some(text) = text {
                    out.push(Block::Clip(text.to_string()));
                }
            }
            ItemKind::Mark | ItemKind::Screenshot => {
                let (label, kind, n) = if item.kind == ItemKind::Mark {
                    marks += 1;
                    ("Mark", "mark", marks)
                } else {
                    screenshots += 1;
                    ("Screenshot", "screenshot", screenshots)
                };
                let file = format!("images/{n:02}-{kind}.png");
                out.push(Block::Image {
                    label: format!("{label} {n}"),
                    bundle_path: bundle_dir.join(&file),
                    file,
                    capture: item.image_path.as_deref().map(PathBuf::from),
                });
            }
        }
    }
    out
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

    let mut images: Vec<PathBuf> = Vec::new();
    let mut total_image_bytes = 0u64;
    let mut warnings: Vec<String> = Vec::new();

    for block in blocks(detail, bundle_dir) {
        match block {
            Block::Text(text) => segments.push(text),
            Block::Clip(text) => {
                let fence = fence_for(&text);
                segments.push(format!("{fence}text\n{text}\n{fence}"));
            }
            Block::Image {
                label,
                bundle_path,
                file,
                ..
            } => {
                let k = images.len() + 1;
                match std::fs::metadata(&bundle_path) {
                    Ok(md) => total_image_bytes += md.len(),
                    Err(_) => warnings.push(format!("Image {k} is missing.")),
                }
                segments.push(match target {
                    // Angle brackets keep a path with spaces a valid link.
                    Target::CodingAgent => format!("![{label}](<{}>)", bundle_path.display()),
                    Target::Chat => format!("[Image {k}]"),
                    Target::Markdown => format!("![{label}]({file})"),
                });
                images.push(bundle_path);
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

/// How an image is referenced in the HTML.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ImageSource {
    /// Inline PNG data URL: what browsers and web apps paste.
    Inline,
    /// `file://` URL: what AppKit's HTML import loads into rich text.
    File,
}

/// The session as HTML, for the rich form of a copy: paragraphs, clips as
/// `<pre>`, and each readable image. An image's file path rides along as its
/// `alt` text and link, so an app that keeps only the text of a paste still
/// has the path. Unreadable images are left out.
pub fn render_html(blocks: &[Block], preface: &str) -> String {
    render_html_with(blocks, preface, ImageSource::Inline)
}

pub fn render_html_with(blocks: &[Block], preface: &str, images: ImageSource) -> String {
    use base64::Engine as _;
    let mut parts: Vec<String> = Vec::new();
    let preface = preface.trim();
    if !preface.is_empty() {
        parts.push(format!("<p>{}</p>", paragraph(preface)));
    }
    for block in blocks {
        match block {
            Block::Text(text) => parts.push(format!("<p>{}</p>", paragraph(text))),
            Block::Clip(text) => parts.push(format!("<pre>{}</pre>", escape(text))),
            Block::Image {
                label, bundle_path, ..
            } => {
                let Some(readable) = block.readable_image() else {
                    continue;
                };
                let src = match images {
                    ImageSource::Inline => match std::fs::read(readable) {
                        Ok(bytes) => format!(
                            "data:image/png;base64,{}",
                            base64::engine::general_purpose::STANDARD.encode(bytes)
                        ),
                        Err(_) => continue,
                    },
                    ImageSource::File => {
                        format!("file://{}", escape(&readable.display().to_string()))
                    }
                };
                let path = escape(&bundle_path.display().to_string());
                parts.push(format!(
                    "<p><a href=\"file://{path}\"><img src=\"{src}\" alt=\"{}: {path}\"></a></p>",
                    escape(label)
                ));
            }
        }
    }
    parts.join("\n")
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn paragraph(text: &str) -> String {
    escape(text).replace('\n', "<br>")
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
    fn coding_agent_links_wrap_paths_in_angle_brackets() {
        let dir = bundle().join("My Sessions");
        std::fs::create_dir_all(dir.join("images")).expect("dir");
        std::fs::write(dir.join("images/01-mark.png"), b"png").expect("image");
        let d = detail(vec![item(
            ItemKind::Mark,
            1.0,
            None,
            Some("captures/m.png"),
        )]);
        let rendered = assemble(&d, Target::CodingAgent, "", &dir);
        let path = dir.join("images/01-mark.png");
        assert_eq!(rendered.text, format!("![Mark 1](<{}>)", path.display()));
    }

    #[test]
    fn html_escapes_text_and_clips() {
        let d = detail(vec![
            item(ItemKind::Speech, 1.0, Some("a < b & c\nnext"), None),
            item(ItemKind::Clip, 2.0, Some("<div>`x`</div>"), None),
        ]);
        let html = render_html(&blocks(&d, Path::new("/bundle")), "");
        assert_eq!(
            html,
            "<p>a &lt; b &amp; c<br>next</p>\n<pre>&lt;div&gt;`x`&lt;/div&gt;</pre>"
        );
    }

    #[test]
    fn html_inlines_existing_images_with_their_path() {
        let dir = bundle();
        std::fs::write(dir.join("images/01-mark.png"), b"png").expect("image");
        let d = detail(vec![item(
            ItemKind::Mark,
            1.0,
            None,
            Some("captures/m.png"),
        )]);
        let html = render_html(&blocks(&d, &dir), "");
        let path = dir.join("images/01-mark.png").display().to_string();
        assert!(
            html.contains("src=\"data:image/png;base64,cG5n\""),
            "{html}"
        );
        assert!(html.contains(&format!("alt=\"Mark 1: {path}\"")), "{html}");
        assert!(html.contains(&format!("href=\"file://{path}\"")), "{html}");
    }

    #[test]
    fn html_falls_back_to_the_capture_before_the_bundle_exists() {
        let dir = bundle();
        let capture = dir.join("capture.png");
        std::fs::write(&capture, b"png").expect("capture");
        let d = detail(vec![item(
            ItemKind::Mark,
            1.0,
            None,
            Some(capture.to_str().unwrap()),
        )]);
        let html = render_html(&blocks(&d, &dir.join("not-written")), "");
        assert!(html.contains("base64,cG5n"), "{html}");
    }

    #[test]
    fn html_for_rich_text_points_at_the_image_file() {
        let dir = bundle();
        std::fs::write(dir.join("images/01-mark.png"), b"png").expect("image");
        let d = detail(vec![item(
            ItemKind::Mark,
            1.0,
            None,
            Some("captures/m.png"),
        )]);
        let html = render_html_with(&blocks(&d, &dir), "", ImageSource::File);
        let path = dir.join("images/01-mark.png").display().to_string();
        assert!(html.contains(&format!("src=\"file://{path}\"")), "{html}");
        assert!(!html.contains("base64"), "{html}");
    }

    #[test]
    fn html_skips_unreadable_images() {
        let d = detail(vec![
            item(ItemKind::Speech, 1.0, Some("words"), None),
            item(ItemKind::Mark, 2.0, None, Some("/nowhere/m.png")),
        ]);
        let html = render_html(&blocks(&d, Path::new("/bundle")), "");
        assert_eq!(html, "<p>words</p>");
    }

    #[test]
    fn html_with_no_preface_starts_with_the_first_block() {
        let d = detail(vec![item(ItemKind::Speech, 1.0, Some("hello"), None)]);
        assert_eq!(
            render_html(&blocks(&d, Path::new("/b")), ""),
            "<p>hello</p>"
        );
        assert_eq!(
            render_html(&blocks(&d, Path::new("/b")), "Be brief"),
            "<p>Be brief</p>\n<p>hello</p>"
        );
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
