use crate::types::{ItemKind, Session, SessionDetail};
use crate::RenderedPrompt;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum BundleError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("image error: {0}")]
    Image(#[from] speaktype_capture::ImageError),
}

/// URL-safe slug for a session name: NFKD → ASCII, lowercase, non-alphanumeric
/// runs → '-', trimmed, at most 48 chars cut at a '-' boundary, never empty.
pub fn slug(name: &str) -> String {
    use unicode_normalization::UnicodeNormalization;

    let mut mapped = String::with_capacity(name.len());
    for c in name.nfkd().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            mapped.push(c);
        } else {
            mapped.push('-');
        }
    }

    // Collapse '-' runs and trim the ends.
    let mut collapsed: String = String::with_capacity(mapped.len());
    let mut in_dash = false;
    for c in mapped.chars() {
        if c == '-' {
            in_dash = true;
        } else {
            if in_dash {
                collapsed.push('-');
                in_dash = false;
            }
            collapsed.push(c);
        }
    }
    let trimmed: &str = collapsed.trim_matches('-');

    let chars: Vec<char> = trimmed.chars().collect();
    let mut slug: String = if chars.len() > 48 {
        let cut = chars[..48]
            .iter()
            .rposition(|&c| c == '-')
            .map_or(48, |at| at);
        chars[..cut].iter().collect()
    } else {
        trimmed.to_string()
    };
    if slug.is_empty() {
        slug = "session".into();
    }
    slug
}

/// The session's bundle directory under `root` (default
/// `~/Documents/SpeakType/Sessions`, passed by the caller):
/// `root/<yyyy-mm-dd of started_at, local tz>-<slug>`. A directory already
/// taken by a different session id (marker file `.session-id`) gets a
/// `-2`, `-3`… suffix; the same session always gets the same directory.
pub fn bundle_dir(root: &Path, session: &Session) -> PathBuf {
    dir_for(root, session, true)
}

/// The session's bundle directory without touching the filesystem: an
/// existing marker wins, otherwise the first free candidate is returned
/// uncreated, so read-only renders compute paths against it.
pub fn bundle_dir_readonly(root: &Path, session: &Session) -> PathBuf {
    dir_for(root, session, false)
}

fn dir_for(root: &Path, session: &Session, allocate: bool) -> PathBuf {
    let date = chrono::DateTime::from_timestamp_millis(session.started_at_ms)
        .map(|dt| {
            dt.with_timezone(&chrono::Local)
                .format("%Y-%m-%d")
                .to_string()
        })
        .unwrap_or_else(|| "1970-01-01".into());
    let base = format!("{}-{}", date, slug(&session.name));

    let mut suffix: u32 = 1;
    let mut candidate = root.join(&base);
    loop {
        if marker_id(&candidate) == Some(session.id) {
            return candidate;
        }
        if !candidate.exists() {
            if allocate {
                // Best-effort allocation: creation failures surface in
                // `write_bundle`, which creates and writes with real errors.
                let _ = std::fs::create_dir_all(&candidate);
                let _ = std::fs::write(candidate.join(".session-id"), session.id.to_string());
            }
            return candidate;
        }
        suffix += 1;
        candidate = root.join(format!("{base}-{suffix}"));
    }
}

/// Materialise the bundle: downscaled `images/NN-<kind>.png`, `prompt.md`,
/// `session.json`, each via temp file + rename; images of items that are no
/// longer live are removed.
pub fn write_bundle(
    root: &Path,
    detail: &SessionDetail,
    rendered: &RenderedPrompt,
) -> Result<PathBuf, BundleError> {
    let dir = bundle_dir(root, &detail.session);
    let images_dir = dir.join("images");
    std::fs::create_dir_all(&images_dir)?;

    let mut keep: Vec<PathBuf> = Vec::new();
    let mut mark_n: u32 = 0;
    let mut screenshot_n: u32 = 0;
    for item in &detail.items {
        let (n, kind) = if item.kind == ItemKind::Mark {
            mark_n += 1;
            (mark_n, "mark")
        } else if item.kind == ItemKind::Screenshot {
            screenshot_n += 1;
            (screenshot_n, "screenshot")
        } else {
            continue;
        };
        let target = images_dir.join(format!("{n:02}-{kind}.png"));
        keep.push(target.clone());
        let Some(src) = item.image_path.as_deref().map(Path::new) else {
            continue;
        };
        // A missing raw capture is already named in `rendered.warning`;
        // the rest of the bundle still materialises.
        if !src.is_file() {
            continue;
        }
        speaktype_capture::downscale_png(src, &target, 1568)?;
    }

    for entry in std::fs::read_dir(&images_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "png") && !keep.contains(&path) {
            std::fs::remove_file(&path)?;
        }
    }

    write_atomic(&dir.join("prompt.md"), rendered.text.as_bytes())?;
    let meta = BundleJson {
        schema_version: 1,
        session: &detail.session,
        items: &detail.items,
    };
    write_atomic(
        &dir.join("session.json"),
        serde_json::to_string_pretty(&meta)?.as_bytes(),
    )?;
    Ok(dir)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BundleJson<'a> {
    schema_version: u32,
    session: &'a Session,
    items: &'a [crate::types::Item],
}

/// Write `bytes` to `path` via a temp sibling + rename so readers never see a
/// partial file.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), BundleError> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn marker_id(dir: &Path) -> Option<i64> {
    std::fs::read_to_string(dir.join(".session-id"))
        .ok()?
        .trim()
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assemble::assemble;
    use crate::store::Store;
    use crate::types::{NewItem, Target};
    use serde_json::json;

    const NOV_19_2026_MS: i64 = 1_795_046_400_000; // 2026-11-19T00:00Z

    fn tempdir() -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "speaktype_bundle_test_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).expect("tempdir");
        dir
    }

    fn session(id: i64, name: &str) -> Session {
        Session {
            id,
            name: name.into(),
            started_at_ms: NOV_19_2026_MS,
            ended_at_ms: None,
            template: "auto".into(),
            target: Target::CodingAgent,
            preface: None,
            bundle_path: None,
        }
    }

    fn write_png(path: &Path, w: u32, h: u32) {
        let img = image_placeholder(path, w, h);
        img.save_with_format(path, image::ImageFormat::Png)
            .expect("png");
    }

    fn image_placeholder(path: &Path, w: u32, h: u32) -> image::RgbImage {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("png parent");
        }
        image::RgbImage::from_fn(w, h, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 3])
        })
    }

    /// A session with two marks backed by real PNGs, via the store.
    fn detail_with_two_marks(root: &Path) -> (Store, crate::types::SessionDetail) {
        let store = Store::open_in_memory().expect("store");
        let s = store
            .create_session("Checkout bug", "auto", NOV_19_2026_MS)
            .expect("session");
        let captures = root.join("captures");
        write_png(&captures.join("m1.png"), 320, 200);
        write_png(&captures.join("m2.png"), 240, 240);
        let mark = |image: String| NewItem {
            kind: ItemKind::Mark,
            t_start_ms: 0,
            t_end_ms: None,
            text: None,
            image_path: Some(image),
            source_app: None,
            payload: json!({}),
        };
        let m1 = store
            .add_item(
                s.id,
                mark(captures.join("m1.png").to_string_lossy().into_owned()),
            )
            .expect("mark 1");
        let m2 = store
            .add_item(
                s.id,
                mark(captures.join("m2.png").to_string_lossy().into_owned()),
            )
            .expect("mark 2");
        // Marks were added in ord order: m1 before m2.
        assert!(m1.ord < m2.ord);
        let detail = store.detail(s.id).expect("detail");
        (store, detail)
    }

    #[test]
    fn slug_handles_unicode_and_length() {
        assert_eq!(slug("Café — checkout bug!!"), "cafe-checkout-bug");
        let long = slug(&"a b ".repeat(200));
        assert!(
            long.chars().count() <= 48,
            "slug must be ≤ 48 chars: {long}"
        );
        assert!(
            !long.starts_with('-') && !long.ends_with('-'),
            "slug: {long}"
        );
        assert!(long.chars().all(|c| c == '-' || c.is_ascii_alphanumeric()));
        assert_eq!(slug("😀"), "session");
        assert_eq!(slug(""), "session");
    }

    #[test]
    fn bundle_dir_readonly_creates_nothing() {
        let root = tempdir();
        let a = session(7, "Checkout bug");

        let dir = bundle_dir_readonly(&root, &a);
        assert!(
            !dir.exists(),
            "the read-only lookup must not create anything"
        );
        assert_eq!(
            dir,
            root.join(format!("{}-checkout-bug", local_date(NOV_19_2026_MS)))
        );
        // Allocation lands on the same path the render computed.
        assert_eq!(bundle_dir(&root, &a), dir);
        assert!(dir.exists(), "allocation creates it");
        // After allocation the read-only lookup resolves the marker.
        assert_eq!(bundle_dir_readonly(&root, &a), dir);
    }

    #[test]
    fn bundle_dir_suffixes_on_collision() {
        let root = tempdir();
        let a = session(1, "Checkout bug");
        let b = session(2, "Checkout bug");

        let dir_a = bundle_dir(&root, &a);
        assert_eq!(
            dir_a,
            root.join(format!("{}-checkout-bug", local_date(NOV_19_2026_MS)))
        );
        assert_eq!(marker_id(&dir_a), Some(1), "marker written at allocation");

        let dir_b = bundle_dir(&root, &b);
        assert_eq!(
            dir_b,
            dir_a.with_file_name(format!("{}-checkout-bug-2", local_date(NOV_19_2026_MS)))
        );

        // The same session keeps its directory on every call.
        assert_eq!(bundle_dir(&root, &a), dir_a);
        assert_eq!(bundle_dir(&root, &b), dir_b);
    }

    fn local_date(ms: i64) -> String {
        chrono::DateTime::from_timestamp_millis(ms)
            .expect("ms in range")
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d")
            .to_string()
    }

    fn marker_id(dir: &Path) -> Option<i64> {
        std::fs::read_to_string(dir.join(".session-id"))
            .ok()?
            .trim()
            .parse()
            .ok()
    }

    #[test]
    fn write_bundle_is_atomic() {
        let root = tempdir();
        let (store, detail) = detail_with_two_marks(&root);
        let dir = bundle_dir(&root, &detail.session);
        let rendered = assemble(&detail, Target::CodingAgent, "", &dir);

        let out = write_bundle(&root, &detail, &rendered).expect("write_bundle");
        assert_eq!(out, dir);

        assert_eq!(
            std::fs::read_to_string(dir.join("prompt.md")).expect("prompt.md"),
            rendered.text
        );
        let meta: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.join("session.json")).expect("session.json"),
        )
        .expect("valid json");
        assert_eq!(meta["schemaVersion"], json!(1));
        assert_eq!(meta["session"]["id"], json!(detail.session.id));
        assert_eq!(meta["items"].as_array().map(Vec::len), Some(2));
        assert!(dir.join("images/01-mark.png").is_file());
        assert!(dir.join("images/02-mark.png").is_file());

        let mut tmp_files = Vec::new();
        collect_tmp(&dir, &mut tmp_files);
        assert!(
            tmp_files.is_empty(),
            "no temp files may remain: {tmp_files:?}"
        );
        let _ = store;
    }

    fn collect_tmp(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("read dir") {
            let entry = entry.expect("entry");
            let path = entry.path();
            if path.is_dir() {
                collect_tmp(&path, out);
            } else if path.extension().is_some_and(|e| e == "tmp") {
                out.push(path);
            }
        }
    }

    #[test]
    fn rewrite_after_delete_removes_stale_image() {
        let root = tempdir();
        let (store, detail) = detail_with_two_marks(&root);
        let dir = bundle_dir(&root, &detail.session);
        let rendered = assemble(&detail, Target::CodingAgent, "", &dir);
        write_bundle(&root, &detail, &rendered).expect("first write");
        assert!(dir.join("images/02-mark.png").is_file());

        // Delete the FIRST mark: the survivor renumbers to 01, so the old
        // 02-mark.png is stale and 01-mark.png must be overwritten.
        let first = detail.items[0].id;
        store.delete_items(&[first]).expect("delete");
        let detail = store.detail(detail.session.id).expect("detail");
        let rendered = assemble(&detail, Target::CodingAgent, "", &dir);
        write_bundle(&root, &detail, &rendered).expect("rewrite");

        let images: Vec<String> = std::fs::read_dir(dir.join("images"))
            .expect("images dir")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            images,
            ["01-mark.png"],
            "only the surviving (renumbered) image may remain"
        );
    }

    #[test]
    fn write_bundle_skips_missing_source_images() {
        let root = tempdir();
        let store = Store::open_in_memory().expect("store");
        let s = store
            .create_session("S", "auto", NOV_19_2026_MS)
            .expect("session");
        store
            .add_item(
                s.id,
                NewItem {
                    kind: ItemKind::Mark,
                    t_start_ms: 0,
                    t_end_ms: None,
                    text: None,
                    image_path: Some(
                        root.join("captures/gone.png")
                            .to_string_lossy()
                            .into_owned(),
                    ),
                    source_app: None,
                    payload: json!({}),
                },
            )
            .expect("mark");
        let detail = store.detail(s.id).expect("detail");
        let dir = bundle_dir(&root, &detail.session);
        let rendered = assemble(&detail, Target::CodingAgent, "", &dir);
        assert_eq!(rendered.warning.as_deref(), Some("Image 1 is missing."));

        write_bundle(&root, &detail, &rendered).expect("bundle still written");
        assert!(dir.join("prompt.md").is_file());
        assert!(!dir.join("images/01-mark.png").exists());
    }

    #[test]
    fn detail_items_in_write_bundle_have_absolute_image_paths() {
        // Documentation-grade assertion of the storage contract Task 12 relies
        // on: raw capture paths are absolute.
        let root = tempdir();
        let (_, detail) = detail_with_two_marks(&root);
        for item in &detail.items {
            let path = item.image_path.as_deref().expect("mark has an image");
            assert!(
                Path::new(path).is_absolute(),
                "image_path must be absolute: {path}"
            );
        }
    }

    #[test]
    fn bundle_json_schema_field_names() {
        // session.json uses camelCase like every serialized session type.
        let root = tempdir();
        let (store, detail) = detail_with_two_marks(&root);
        let dir = bundle_dir(&root, &detail.session);
        let rendered = assemble(&detail, Target::CodingAgent, "", &dir);
        write_bundle(&root, &detail, &rendered).expect("write");
        let raw = std::fs::read_to_string(dir.join("session.json")).expect("read");
        assert!(raw.contains("\"schemaVersion\""));
        assert!(raw.contains("\"startedAtMs\""));
        assert!(raw.contains("\"sessionId\""));
        let _ = store;
    }

    #[test]
    fn screenshot_images_use_their_kind_in_file_and_alt() {
        // The image file name is NN-<kind>.png with kind from ItemKind.
        let root = tempdir();
        let store = Store::open_in_memory().expect("store");
        let s = store
            .create_session("S", "auto", NOV_19_2026_MS)
            .expect("session");
        write_png(&root.join("captures/s1.png"), 100, 100);
        store
            .add_item(
                s.id,
                NewItem {
                    kind: ItemKind::Screenshot,
                    t_start_ms: 0,
                    t_end_ms: None,
                    text: None,
                    image_path: Some(root.join("captures/s1.png").to_string_lossy().into_owned()),
                    source_app: None,
                    payload: json!({}),
                },
            )
            .expect("screenshot");
        let detail = store.detail(s.id).expect("detail");
        let dir = bundle_dir(&root, &detail.session);
        let rendered = assemble(&detail, Target::Markdown, "", &dir);
        write_bundle(&root, &detail, &rendered).expect("write");
        assert!(dir.join("images/01-screenshot.png").is_file());
        assert!(rendered
            .text
            .contains("![Screenshot 1](images/01-screenshot.png)"));
    }
}
