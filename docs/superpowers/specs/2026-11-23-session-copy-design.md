# Copying a session: one copy for every app, no persona by default

Date: 2026-11-23. Approved in conversation with the owner.

## Why

Sessions are pasted mostly into coding agents (Claude Code, Cursor), sometimes into notes and docs. Today a copy is plain text with absolute image paths and always starts with a guessed persona ("You are a research assistant…"). The owner wants:

- the screenshots themselves on the clipboard, like copying part of a web page;
- the app being pasted into to pick the form it understands, with no mode to choose;
- no persona unless asked for; when asked, a choice of persona.

## Decisions

1. **One copy, several forms.** The Copy button writes, in one pasteboard item:
   - plain text: the words, with each image as `![Mark 1](</absolute/path.png>)` (angle brackets keep paths with spaces valid). Terminals and coding agents read this; agents open the files themselves.
   - HTML: the words as paragraphs, clips as `<pre>`, each image inline as a PNG data URL, with its file path as the `alt` text and a `file://` link, so a reader that strips images still keeps the path. Browsers, Docs and Slack read this.
   - RTFD: rich text with the images attached, made from the same HTML. Notes, Mail and Pages read this.
   - A private marker type, `com.speaktype.session-copy`, so SpeakType's clipboard watcher never records its own copy as a clip.
2. **Persona: "Start with", default Nothing.** A new setting `session_copy_preface` with values `none` (default), `auto` (pick from what was said; the old behaviour) or a persona key (`general`, `thesis`, `qa_test_plan`, `blog`, `meeting_notes`). A preface typed for a single session still wins. With Nothing, the copy starts with the first word or image, not even a blank line. Document generation (`llm.rs`) keeps using personas as before.
3. **The saved `prompt.md` uses relative links** (`images/01-mark.png`) so it previews in any Markdown viewer. Already built.
4. **Images come from the bundle** (downscaled, at most 1568 px). If a copy happens before the bundle is written, the capture file is used instead.

## Shape of the code

- `speaktype-session::assemble` builds one ordered list of blocks (text, clip, image with label and path). The existing text targets and a new `render_html` both render from that list, so the forms can never disagree.
- The app's `prompt_copy` writes the pasteboard through `NSPasteboard` (macOS). It is one command; the frontend's Copy button is unchanged.
- Settings › Sessions gets a "Copying" section with the "Start with" menu.

## Non-goals

- No copy-mode setting: the pasting app chooses. A setting comes back only if a real app is found that picks the wrong form.
- No JPEG re-encoding or size caps beyond the existing downscale.

## Verification

Unit tests for blocks, HTML rendering and preface resolution. Then a manual paste of a real session into Claude Code (terminal), Cursor's chat, Notes and a browser text field, and the clipboard watcher checked not to record the copy.
