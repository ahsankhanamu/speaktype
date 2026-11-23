# Session Copy (one copy for every app) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The session Copy button writes plain text with image paths, HTML with inline images and RTFD in one pasteboard item, starts with no persona unless the "Start with" setting asks for one, and is never re-captured as a clip.

**Architecture:** `speaktype-session::assemble` builds ordered blocks once; text and HTML render from them. The app writes the pasteboard with AppKit (`NSPasteboard`, `NSAttributedString` HTML → RTFD). A new setting picks the preface.

**Tech Stack:** Rust (speaktype-session, Tauri app), objc2-app-kit 0.3, Svelte 5 settings page.

**Spec:** docs/superpowers/specs/2026-11-23-session-copy-design.md

## Global Constraints

- Preface default is nothing: the copy starts with the first block, no leading newline.
- Persona keys: `none` (default), `auto`, `general`, `thesis`, `qa_test_plan`, `blog`, `meeting_notes`. A per-session `preface` wins over the setting.
- Image paths in plain text are wrapped as `<…>`.
- Marker pasteboard type: `com.speaktype.session-copy`.
- No new copy-mode setting.

## Review Focus

- A session with no images: plain text and HTML only words, no empty `<img>`.
- An image path containing spaces: plain text link stays valid (`<…>`), HTML `src` is a data URL so spaces don't matter.
- A clip containing `<`, `&` or backticks: HTML escapes it; text keeps its fence.
- Copy before the bundle is written: images come from the capture file; missing files are skipped in HTML, still named in the warning.
- The clipboard watcher sees the copy's plain text: it must skip it because of the marker type.

---

### Task 1: Blocks and HTML in speaktype-session

**Files:** Modify `crates/speaktype-session/src/assemble.rs`, `crates/speaktype-session/src/lib.rs`.

**Interfaces — Produces:**
- `pub enum Block { Text(String), Clip(String), Image { label: String, path: PathBuf } }`
- `pub fn blocks(detail: &SessionDetail, bundle_dir: &Path) -> Vec<Block>` — order of items; image path is the bundle file if it exists, else the item's `image_path`.
- `pub fn render_html(blocks: &[Block], preface: &str) -> String` — `<p>` per text (newlines → `<br>`), `<pre>` per clip, `<p><a href="file://…"><img src="data:image/png;base64,…" alt="<path>"></a></p>` per readable image; all text HTML-escaped.
- `assemble` keeps its signature and output, built from `blocks`; CodingAgent links become `![Mark 1](<path>)`.

- [ ] Tests: `html_escapes_text_and_clips`, `html_inlines_existing_images_with_path_alt`, `html_skips_unreadable_images`, `coding_agent_links_wrap_paths_in_angle_brackets`, `no_preface_starts_with_first_block`. Run `cargo test -p speaktype-session`, see them fail, implement, see them pass.
- [ ] Update existing tests that pin the old CodingAgent link form.

### Task 2: "Start with" setting and preface resolution

**Files:** Modify `apps/widget/src-tauri/src/settings.rs`, `session_cmds.rs`, `apps/widget/webui/src/app-window/pages/Sessions.svelte`, regenerate `apps/widget/webui/src/lib/bindings.ts`.

**Interfaces:** `Settings.session_copy_preface: String` (serde default `"none"`). `fn resolve_preface(detail: &SessionDetail, choice: &str) -> String`: session preface if set; `none` → `""`; `auto` → `templates::template_prompt(templates::infer_template(text))`; key → `templates::template_prompt(key)`.

- [ ] Tests: `preface_none_is_empty`, `preface_persona_uses_its_prompt`, `session_preface_wins`, and settings round-trip default `"none"`.
- [ ] UI: a "Copying" section with a `PopupMenu` "Start with": Nothing · Pick for me · Research · Thesis · QA test plan · Blog · Meeting notes.

### Task 3: Write the pasteboard and skip it in the watcher

**Files:** Create `apps/widget/src-tauri/src/rich_copy.rs`; modify `session_cmds.rs` (`prompt_copy`), `clipboard.rs`, `Cargo.toml` (objc2-app-kit features `NSPasteboard`, `NSAttributedString`), `lib.rs`.

**Interfaces:** `pub fn write_session_copy(text: &str, html: &str) -> Result<(), String>` (macOS: clear, set plain text, HTML, RTFD from `NSAttributedString::initWithHTML_documentAttributes`, marker; elsewhere: text only). `pub fn is_session_copy() -> bool` reads the marker. `prompt_copy` renders blocks once, text for the session's target, HTML, then calls `write_session_copy`.

- [ ] Test (macOS): `write_then_read_back_has_text_html_and_marker` against a private named pasteboard (never the user's).
- [ ] Watcher: skip a change while `is_session_copy()`.

### Task 4: Verify

- [ ] `make verify` passes.
- [ ] Build, install (settings backed up), paste a real session into Claude Code, Cursor chat, Notes and a browser field; check the watcher log records no clip.
