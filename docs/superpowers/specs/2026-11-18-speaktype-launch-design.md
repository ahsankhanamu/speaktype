# SpeakType Launch — Product & Architecture Design

**Date:** 2026-11-18
**Status:** Draft for approval
**Owner:** Ahsan (product) · Claude (staff eng / design / reviewer)
**Supersedes for product direction:** `docs-local/ADR-002-scribe-feature-detailed-design.md` (data-layer details there remain valid unless changed below)

---

## 1. Intent

Take SpeakType from a polished open-source v0.1 to a product people love enough to recommend and, later, pay for. It must remove real daily friction, feel premium, and be effortless for non-technical people while delighting developers.

**Positioning:** *Say it. Show it. Send it.*
Talk and draw over your live screen; when you stop, a ready-to-send prompt (text + annotated screenshots) is waiting for your coding agent, bug tracker, or notes.

### Who it's for
Anyone who works at a computer and writes, builds, reviews, or takes notes: the general public, developers, testers/QA, vibe coders, reviewers, note-takers. Consequence: **zero jargon on the default path**; power options live under *Advanced*.

### Pain points we solve (priority order)
1. **"Describing a visual problem in text is slow and lossy."** → Show & Tell sessions: talk + draw on the live screen → multimodal prompt. *(signature)*
2. **"I won't send my voice to the cloud."** → Local Whisper, no account, no telemetry.
3. **"Typing long prompts/messages is slow."** → Hotkey dictation into any app (press to start, press to stop — today's toggle behaviour). *(daily habit, free hook)*
4. **"Setup is a maze."** → First successful dictation within ~60 s of first launch.
5. **"Did it hear me? Where did my text go?"** → Clear live state, recovery (re-paste/copy), useful history.
6. **"My context is scattered across voice, screenshots, and copied text."** → One session timeline that holds all of it.

## 2. Goals, non-goals, anti-goals

**Goals (launch = "B": app + marketing website)**
- Show & Tell sessions: click-through drawing overlay, continuous narration, auto-captured annotated screenshots, copied-text capture.
- History window with **Timeline** and **Prompt** views; one-keystroke copy of a prompt usable by Claude Code, opencode, Codex, Gemini CLI, Cursor.
- Dictation loop polished to commercial quality, including recovery.
- Rebuilt onboarding (≤ 60 s to first dictation), restructured Settings.
- A real component library on the existing tokens; every screen has loading/empty/error states, both themes, keyboard access.
- Auto-update, Developer-ID signing + notarization.
- Marketing website (static) with demo, features, privacy story, download.
- Visible-but-inert Pro surfaces so licensing slots in later without redesign.

**Non-goals (launch)**
- Payments/licensing (phase "C", next).
- Direct send to running agents / ACP / agent detection (later phase; the session bundle format is designed for it).
- Windows/Linux GUI (Python CLI remains for them).
- OCR text inside prompts. **Screenshots go into prompts as images; agents interpret them.** OCR stays only as a background search index.
- LLM restructuring as part of prompt assembly (assembly is deterministic). Text cleanup modes are a later Pro feature.

**Anti-goals (failure even if it "works")**
- The overlay ever blocks normal clicking/scrolling when the draw key isn't held.
- Any user-visible jargon on the default path (GGUF, llama-server, base URL, Whisper model sizes without plain labels).
- Stop → prompt takes noticeable time, or requires a model download.
- Dictation pastes while a session is narrating.
- A screen that looks "developer settings panel" rather than product.

## 3. Constraints
- macOS 14+ (ScreenCaptureKit features in use), Apple Silicon first.
- Stack stays: Tauri 2 + Rust; Svelte 5 + Vite webui; bits-ui (shadcn-svelte style primitives); lucide icons.
- **Visual direction: native macOS** (decided in design review 2026-11-18, replacing the earlier zinc/Tailwind look the owner called "vibe coded"): macOS window greys (#1E1E1E window, #2A2A2C groups, #3A3A3C controls), system blue #0A84FF for actions and focus, orange #EF643C only for ink/live, **amber #FF9F0A for warnings/attention (never yellow; yellow is reserved for the highlighter ink)**, SF Pro system font only, System-Settings-style grouped rows with hairline separators, real keycaps, status as dots not pills, no filled banners, composable shadcn-style components (bits-ui) never native `<select>`. **Every floating layer (Select content, menus, popovers, tooltips, toasts) renders through a Portal at the window root** with a z-index scale (`--z-popover`, `--z-toast`), never inside its row, so it is never clipped or covered. Radius: 6 controls, 12 grouped cards, 10 popovers, 12 windows. `theme.css` tokens are re-valued to this palette (names kept). **Light mode** (boards "Light mode — Components / Appearance"): Apple light system colours — window #F5F5F7, groups #FFFFFF, controls #E5E5EA, blue #007AFF, warning #FF9500, red #FF3B30, green #34C759; hairlines and shadows about half the dark strength; popover elevation `0 0 0 .5px rgba(0,0,0,.12), 0 4px 12px rgba(0,0,0,.14)`. The floating widget/HUD stays dark in both themes; ink colours and theme previews are theme-independent. **Density and era: current macOS (Tahoe / Liquid Glass), not the older System Settings density** (owner review: "too compact, older"). Content windows use regular density: 14 pt body, 15 pt section titles, 24 pt page titles; capsule controls 32–34 pt (search, segmented, toolbar buttons); floating glass sidebar inset 8 pt with 14 pt radius; list rows ≈ 64 pt with 32 pt icon tiles; cards 18 pt radius; **row height by row type: read-only info rows (label · value) 36 pt at 13 pt text; rows with a control 44–50 pt; rows with an icon tile or two-line text ≈ 64 pt**; a row whose value is a set of thumbnails (e.g. Details → Marks) stays inline while ≤ 3 fit; beyond that it stacks — label (same style as other labels, count + "Show all" trailing) above one horizontally scrolling thumbnail strip, same cell, no separator; thumbnails open Quick Look on click/Space and are labelled "Mark n of N"; window radius 18. The sidebar uses **17 pt labels, 36 pt rows, 24 pt tiles (14 pt glyph), 256 pt wide** by default — owner's explicit choice, one step above the HIG Large size (15 pt) — and steps down 2 pt per level for the system's Medium/Small Sidebar icon size setting. Settings content: 14 pt row labels with 12 pt captions, ≥ 50 pt rows, 16 pt card radius, 15 pt section titles; every button, dropdown trigger, search and shortcut field is a capsule (28 pt in settings rows, 32 pt in panels and toolbars). Same rules for the quick review panel, onboarding and menus. Reference board: History — Details tab. Reference boards: Design language, Onboarding, Settings.
- Privacy promise: no accounts, no telemetry, nothing leaves the machine unless the user configures it.
- Permissions: Microphone, Accessibility (already), Screen Recording (asked just-in-time on first session). Input Monitoring may be required for the draw-key event tap — verify early (see §9 risks).
- Multi-agent build: work must split by file ownership with a fixed IPC contract (§7.4).

## 4. Approaches considered (overlay)

| # | Approach | Trade-off |
|---|---|---|
| **1 ✅** | **Transparent full-screen Tauri webview per display**, NSPanel at the widget's level, Svelte canvas for ink; Rust toggles `ignoresMouseEvents` while the draw key is held | Same stack → fast to polish, agent-friendly. Costs tens of MB per display while a session runs. |
| 2 | Native AppKit/CoreGraphics overlay in Rust (objc2) | Lowest latency/memory; every visual change is slow low-level work; agents weak at it. Gains invisible to most users. |
| 3 | No live overlay; screenshot → markup editor | Simplest; loses the core magic (drawing on the *live* screen while working). |

**Chosen: 1** (owner said "do what is best").

Other decisions (one sensible way; recorded as Claude's calls):
- After a session, work stays in place (HUD + quick review panel); past sessions and dictations live in a separate **History** archive window, not Settings.
- Component library built on bits-ui with the existing tokens.
- Website: SvelteKit + static adapter in `apps/site`.
- Launch scope B; Pro surfaces present but unlocked.

## 5. Product surfaces

```
┌ Widget (always on top) ─ dictation pill; morphs into Session HUD
├ Overlay (per display)  ─ ink layer, only during sessions
├ Quick review panel     ─ small panel attached to the HUD after Stop
└ SpeakType window       ─ ONE window, one sidebar: History · General · Appearance · Dictation · Sessions · Permissions · Storage · Advanced (+ app row → About)
      History is a page in this window (sidebar | results | detail), never a separate orphan window
├ Onboarding window      ─ first run
└ Website (apps/site)
```

**Navigation (owner review: "History looks orphaned"):** everything except the floating widget/HUD/overlay lives in one SpeakType window with one sidebar. Entry points: menu bar → History… (⌘Y) or Settings… (⌘,) opens that window on the matching page; quick review → "Open in History" opens that session's Timeline; sidebar switches pages in place; History's Timeline · Result · Details tabs switch in place. The window remembers the last page.

Menu-bar menu: header "SpeakType" + state ("Ready · Balanced model"); Start dictation ⌃⌘; Start a session ⌥⌘K; Paste last dictation again; History… ⌘Y; Settings… ⌘,; **Hide/Show widget** (exists today, keep); Check for updates…; About SpeakType (exists, keep); Quit ⌘Q. The menu-bar icon keeps its existing 4 states (idle, recording, transcribing, error flash; `tray.rs:20-69`) and adds a 5th: session live (orange). Drop the "ST" text title.

**Key names:** UI says **Return** (↩) per Apple's style guide, never "Enter" (that's the keypad key); internal setting values (`enter`, `enter+enter`) stay as they are. **Shortcut field:** 32 pt capsule recorder with one 24 pt keycap per key (13 pt glyph, raised gradient + 1 px base shadow), hover outline, trailing ✕ to clear; click to record. **Shortcut notation:** always Apple's modifier order ⌃ ⌥ ⇧ ⌘ (e.g. ⌃⌘, ⌥⌘K), as glyphs, in keycaps where a shortcut is the subject.

**Core principle (owner, design review 2026-11-18): people stay in their work.** Nothing after a session or dictation pulls them into a SpeakType window. Copying, quick fixes and re-sending happen from the HUD and its quick review panel, in place. The History window is an archive people open on purpose, for past work and debugging, never automatically.

## 6. Experience design

Designs live in the **SpeakType Launch Designs** artifact (https://claude.ai/artifact/KWazBMcvch9XExYDgWTUoL). Text here is normative; designs are the visual reference.

### 6.1 Dictation (widget)
States: `idle → recording → transcribing → success | error`, as today, refined:
- Hotkey → recording indicator visible < 100 ms **when the model server is ready**; if it isn't (`commands.rs:144-190` loads it first) show a **"Getting ready…"** pill immediately instead of nothing. Tones exist today (`tones.rs`) and always play; a Sounds toggle is new.
- Recording pill: live waveform, timer, cancel (✕), stop. (Esc-to-cancel is **new work**: the widget is a non-activating NSPanel and can't receive keys, so register a temporary global Esc shortcut only while recording.)
- Success: brief ✓ then idle. If paste could not land, show **"Copied. Press ⌘V to paste"** with *Paste again*. Build on what exists: `paste_text` already returns false and the text is already left on the clipboard, and `sidecar:error` carries "Could not paste… The text is on your clipboard" (`paste.rs:334-357`, `commands.rs:525-536`); today the widget ignores the message and shows a generic ✕ (`Widget.svelte:701`).
- Error states use plain language with one action, **one state per existing event**: `sidecar:no_speech` → "Didn't catch that"; `sidecar:busy` → "Still working on the last one"; paste failure (above); `server:error` → "Speech model couldn't start · Try again" (server auto-restarts with a cap, `lib.rs:336-345`); `sidecar:long_audio` (> 120 s) → "Long recording, this may take a moment"; `sidecar:onboarding_required` → opens onboarding (existing behaviour). Microphone unavailable → "Choose mic". Normalise payloads to one `{kind, message}` shape.
- **Microphone picker** (from "Choose mic" or the widget's new right-click menu; also Settings → Dictation). Right-click today opens Settings (`Widget.svelte:626`); it becomes a context menu: Microphone ▸, History, Settings, Hide widget. Keep the existing **System default** choice (`Settings.svelte:933`) and the existing Bluetooth label (`audio.rs:538`). Every input device listed with its **own live level meter** (**new work**: today one `LevelMonitor` stream emits `mictest:level`, `audio.rs:196-283`; open one short-lived stream per device only while the picker is open), so speaking identifies the right mic; selected = check; disconnected devices shown as "Not connected"; Bluetooth devices note the quality trade-off. If the chosen mic disconnects mid-dictation, fall back to the built-in mic, keep recording, and say so in the pill.
- Every dictation lands in **History** (replaces Settings → History) with copy / re-paste / delete / play audio.

### 6.2 Show & Tell session
**Start — always-on drawing (owner decision 2026-11-19):** hold the draw key (Right ⌥) **any time** and draw; the first stroke starts a session automatically (HUD appears, mic goes live with a tone and a visible live indicator; click the level to mute). `⌥⌘S` takes a screenshot any time and joins the current session or starts one. The session hotkey (`⌥⌘K`) and tray still start a session for people who want to talk first. A session ends on Stop / Copy, or by itself after ~2 min with no ink, speech or copied text (saved; HUD shows "Session saved · Copy"). **Settings → Sessions → "Draw anytime"** (on by default) turns always-on drawing off; then drawing works only inside a session started by `⌥⌘K` / tray. Guardrails: the mic never listens while idle — it turns on only when a session starts; only **Right** ⌥ is used (left ⌥, ⌥-click and special characters keep working, and SpeakType never takes the keyboard — only the mouse, only while the key is held); the draw key is changeable (Right ⌘, Fn); idle cost is only the listen-only key tap — the overlay is created on key-down with the first stroke buffered (or one hidden pre-built overlay per display if the Phase 2 spike shows creation is too slow). Screen Recording is requested at the first stroke. Onboarding's "next thing to try" is "Hold Right ⌥ and draw". (History has no prominent "New session" button; it's not where sessions start.) No naming up front — auto-named from first spoken words + time; renamable later. Template defaults to last used.

**Session HUD** (the widget morphs into a wider pill, draggable):
`● 02:14 · [live level = mute toggle] · ✎ 3 · 📋 1 · [✎ Pen] [Stop]` — counts use real 16 pt icons with hover/VoiceOver labels ("3 marks", "1 copied clip"); there is no separate mic button: clicking the live level mutes (it becomes a crossed-out mic). Screenshot without ink stays on ⌥⌘S and the right-click menu.
- Mic is live for the whole session (continuous narration), clearly indicated; Mute toggles it (dictation hotkey also toggles mute during a session). **No pasting during sessions.**
- Narration is segmented by voice-activity detection and transcribed in the background (reuse `chunk_session.rs`), so Stop is instant.
- **Live captions** under the HUD show the latest transcribed segment (fade after ~3 s of silence) so the user knows they're being heard. Toggle in Settings → Sessions; on by default.

**Drawing:**
- **Hold Right ⌥ (default; configurable: Right ⌘, Fn)** → the overlay takes the mouse; ink appears. Release → click-through restored; ink stays.
- **Pen toggle** in HUD for trackpad/pen users: sticky draw mode; Esc or toggle exits.
- Tools (small palette near cursor while key held, minimal): pen (default), arrow, rectangle, highlighter; 3 colours (accent red default, yellow, blue). Undo `⌘Z` while drawing.
- **A "mark" = strokes until 900 ms of no drawing after key release.** Ink fades out (250 ms) when the mark completes. Scrolling never leaves stale ink.
- **How a mark is captured (clean original + vector ink, composited):**
  1. **Freeze-frame at the first stroke.** On draw-key down (or first pen stroke) for a new mark, capture the display under the cursor with ScreenCaptureKit, **excluding every SpeakType window** (overlay, HUD, captions, widget) via the content filter. This is the clean original, captured *before* anything can scroll.
  2. **Ink is vector data**, never pixels: strokes stored in display points with tool, colour, width, timestamps, plus the display id and `backingScaleFactor`.
  3. **Mark completes** after 900 ms idle — or **immediately** if the user scrolls, clicks through, or switches app while a mark is pending — so ink always matches the frozen frame.
  4. **Composite in Rust**: ink scaled points→pixels by the display's scale factor and rendered over the clean frame → `NN-mark.png`. The clean frame (`NN-mark.orig.png`) and strokes (`session.json`) are kept, so ink can be edited/toggled later in the Timeline and the output re-rendered.
  5. **Framing (what the agent sees)**: crop to the **window under the ink** (ScreenCaptureKit window frame at that point) so the agent gets the app with context, not the whole desktop; if ink spans several windows or the desktop, use the union, falling back to the full display. Then downscale to ≤ 1568 px long edge. Original full-res frame is retained on disk.
- **Displays & resolutions:** one overlay per display; each display is its own coordinate space with its own scale factor (Retina and non-Retina mix correctly). A mark belongs to the display its first stroke started on; strokes that cross onto another display start a new mark there. Display hot-plug/resolution changes rebuild the overlays between marks.
- **Never in the image:** the HUD, captions, tool palette, cursor, and SpeakType widget are always excluded — the agent only sees your app plus your ink.
- Optional **Screenshot** action (HUD / `⌥⌘S`): native region picker, no ink.
- **Copied text** while a session is live becomes a *clip* (dedupe; ignores SpeakType's own pastes) — existing behaviour.

**Stop:** HUD Stop / session hotkey → HUD shows **"Session saved · 3 marks · 1 clip"** with **Copy** (primary, uses the session's default output) and **Review**. Never label a session as only a prompt.
- **Review** opens the **quick review panel** attached to the HUD (≈ 420 × 520, floating, does not activate SpeakType as the frontmost app beyond the panel): mark thumbnails with delete, transcript text editable inline, output picker (Coding agent / Chat / Document), template, **Copy**. Esc or Copy closes it. A small "Open in History" link is the only path to the big window.

### 6.3 History window (archive, opened on purpose)
All tabs share one window, one results list, one search and one header; only the tab content changes. Timeline, Result and Details all use the same grouped-card component and spacing.
A page of the SpeakType window, reached from the sidebar, the menu bar (History… ⌘Y), or the quick review panel's "Open in History". Built for lookup and debugging:
- **Toolbar:** title, filter (All · Sessions · Dictations), and a prominent **search** across everything said, copied, captioned, or OCR'd.
- **Results list:** sessions and dictations together, grouped by day; each row shows icon, title, matching snippet, and outcome ("sent to Claude Code", "pasted", "copied").
- **Detail:** header with a **Copy** split button (main part copies in the format last used for this entry; the chevron menu offers Agent prompt / Chat / Notes / Markdown; label flips to "Copied" for 1.5 s), **Show in Finder**, more menu (Export…, Delete). Never "Copy again": an entry may never have been copied (e.g. paste failed). Segmented **Timeline | Result | Details**.
- **Details tab (debugging):** speech model and language, segment count and slowest segment, time from Stop to ready, screens and scale, what each mark was framed to, clips, output and copy time, folder, and a **Problems** list (failed segments, missed captures, paste failures) plus **Copy diagnostics** (no audio or screenshots unless chosen).
- **Report a problem…** (replaces a bare "Copy diagnostics"; reached from History → Details, Settings → Advanced, and the menu bar) opens a **sheet** with: a "What happened?" field; screenshots (the related mark is pre-attached; **Capture screen…** via `screencapture -i`; drag-and-drop files); **Included automatically** with per-item switches — About your Mac (always), this session/dictation's details (the Details data, incl. the debug record), recent app log (last 30 min, with user names and paths scrubbed), audio (off by default, size shown); **Preview everything that will be sent**. Optional *Your email* field (so support can reply). Actions: **Send report** (primary, one click: uploads the description, screenshots and switched-on items as one encrypted bundle over HTTPS to a small SpeakType report endpoint, then shows "Report sent" with a reference like `ST-4F2K-91`, Copy reference, Done; a copy of what was sent is kept in History → Details), **Save report…** (zip, for offline or manual sharing), Cancel. If sending fails (offline), the sheet keeps everything and offers Try again / Save report…. Nothing is ever sent without pressing Send. **New work, incl. a minimal backend** (e.g. a serverless endpoint + object storage that notifies the team; retention policy and privacy-page entry required). This absorbs the old Settings → Debug tab.

**Timeline view** — vertical, time gutter on the left:
- *Speech* item: transcribed text, inline-editable, play snippet.
- *Mark* item: annotated screenshot thumbnail (click → large preview), optional caption.
- *Screenshot* item, *Clip* item (monospace block, source app if known).
- Hover actions: delete, move up/down, drag to reorder, "split here". Multi-select delete.
- Edits are reflected in the Result view immediately.

**Result view** (named neutrally: a session can become an agent prompt, a chat message, notes or a document) — rendered output with inline images exactly as it will be sent or saved:
- **Target**: `Coding agent (file paths)` default · `Chat app (images separately)` · `Markdown export`.
- **Template** preface. Keep the existing templates and ids (`templates.rs:8-57`: general, thesis, qa_test_plan, blog, meeting_notes, and keyword-based **auto**, the current default) and add bug_report, feature_request, code_review. Picker order: Auto, Bug report, Feature request, Code review, QA test plan, Meeting notes, General, then More (Thesis, Blog post) — a short instruction header + optional section framing; user can edit the preface text.
- Primary: **Copy prompt** (`⌘↵`). Secondary: Copy images · Reveal folder · Export .md / .docx.
- Size hint: "7 images · ~4.1 MB" with a warning above 20 images.

### 6.4 Prompt assembly (deterministic, local, instant)
Ordered interleave of timeline items (by `ord`, default chronological):
```
<template preface>

<speech text>
![Mark 1](/Users/<u>/SpeakType/Sessions/2026-11-18-checkout-bug/images/01-mark.png)
<speech text>
```text
<clip text>
```
```
- *Coding-agent target*: absolute file paths (Claude Code, opencode, Codex, Gemini CLI, Cursor read them).
- *Chat target*: text with `[Image 1]` placeholders + "Copy images" puts PNGs on the pasteboard (multi-item) / drag-out.
- Images downscaled to ≤ 1568 px long edge, PNG.

### 6.5 Session bundle on disk (stable contract; future direct-send uses it)
```
~/Documents/SpeakType/Sessions/<yyyy-mm-dd>-<slug>/
  prompt.md          # regenerated on change
  session.json       # metadata + ordered items (schema-versioned)
  images/01-mark.png …
  audio/ (optional, setting)
```
SQLite (`scribe.db`) remains the source of truth; the bundle is materialised on Stop and on edits (debounced).

### 6.6 Onboarding (≤ 60 s to first dictation)
**Speech model step:** a Faster ↔ More accurate slider with three stops (Fast / Balanced / Accurate), pre-set to the catalog's recommendation for this Mac (shown: model · chip · RAM). Choosing starts the download immediately; Fast (~150 MB) is fetched first so Try it works at once, and the chosen model loads when ready (existing queue + `load_after`). Continue is never blocked by a download; progress stays in the footer. "See all models…" opens the same Speech models sheet.
**Presentation:** one 960 × 640 window, centred; steps 1–5 are pages of that window (Welcome, Microphone, Accessibility, **Speech model**, Try it) that slide in from the right (reduced-motion: cross-fade), with Back / Continue bottom-right and the step label ("Step 1 of 4") top-left — Welcome is unnumbered. Continue unlocks only when the step is done. Accessibility auto-detects the grant and advances by itself; Microphone detects the grant and then shows the device picker with live levels, so it waits for Continue. Recovery help (stale entries, "Grant to:", Quit & Reopen) is progressive: it appears behind a "Not working?" link after 15 s without a grant, or at once on dev builds. Model choices use `load_model` (queues with load-after and switches when ready). Quitting midway resumes at the same step on next launch; until finished, the widget shows a small "Finish setup" dot that reopens onboarding. Finishing closes the window and shows the existing "You're all set" widget card. Step 6 (Screen Recording) is not part of the window: it's a just-in-time sheet at the first session start.
1. **Welcome** — one line + looping demo; "Get started".
2. **Microphone** — permission with live level meter confirming it works. Fast model starts downloading in the background now; Balanced follows automatically.
3. **Accessibility** — why (to type into other apps), deep-link to System Settings, auto-detect grant.
4. **Try it** — practice text field; "Press ⌃⌘ and say something" (the configured dictation hotkey, rendered as keycaps). Success celebrates; shows the session hotkey as "next thing to try".
Onboarding screens (designs: "Onboarding — Welcome · Microphone · Accessibility" and "Try it"): Welcome (unnumbered) → Step 1 Microphone (every input device as a large choice with its own live level — "the bars move on the microphone that hears you"; Bluetooth quality note; model download starts quietly in the footer) → Step 2 Accessibility (mock of the System Settings list showing exactly which switch to turn on; auto-advances when granted, Continue disabled until then) → Step 3 Try it, which shows the **Speech model** in use as a dropdown: *On this Mac* models with their state (ready / ready in N min / Download · size) and, when a cloud provider is configured, a *Cloud* group with that provider's models (otherwise "Add a cloud service…"). Picking another model **switches the active model, then re-runs the last practice sentence** with it, and offers *Keep <model>* / *Back to <previous>*.
    - **How models actually load (verified in code and on a real M3 Pro / 18 GB):** one `whisper-server` sidecar holds exactly one model. Switching = `restart_server_with_model` (`commands.rs:923`): phases `preparing → stopping → starting → loading → done`, guarded by `MODEL_OPERATION_IN_PROGRESS` so only one switch runs at a time and dictation waits (`commands.rs:151`). Load time from real logs: ~2–4 s warm, ~16 s cold, 60 s timeout (`server.rs:20,231-260`). `reprocess_history_entry` (`commands.rs:1312`) always uses the *active* model, so any compare is switch-then-reprocess, not side by side.
    - **Copy follows that truth:** menu states are *In use*, *Downloaded*, *Downloading · N%* (selecting it queues `load_after`, "switches when ready"), *Download · size*; while switching, the widget and the row show **"Getting ready…"** (the existing `loading` phase message "Loading model into memory" becomes "Loading Balanced… a few seconds"); on timeout: "Balanced couldn't start. Using Fast." (server restarts with the previous model). Never claim "ready in 1 min".
Screen Recording is requested just-in-time on first session start with a one-screen sheet: what it's for ("only the moment you draw, never video, stays on this Mac"), three numbered steps, **Not now** (session continues with voice + copied text) and **Open System Settings** (session starts automatically on return).

**Empty states** teach the next action (e.g. History: "Nothing here yet" + both shortcuts), never a blank pane.

### 6.7 Settings (restructured, macOS System Settings pattern)
**Sidebar** (256 pt; owner-chosen, above HIG Large): 36 pt rows, 17 pt labels, 24 pt icon tiles (6 pt radius, 14 pt glyph), 10 pt icon-to-label gap, 10 pt row inset, 8 pt selection radius, 2 pt between rows, 10 pt between groups; hover = rgba(255,255,255,.07) fill fading in over 120 ms; selected = accent fill with white label (no hover change). Search field at top (24 pt); app identity row (icon, "SpeakType", version + update status, opens About/updates); then grouped items with coloured icon tiles and spacing between groups; selected item = accent fill.
- **Search** filters across *individual settings*, not just page names (e.g. "draw" → Sessions › Draw while holding), showing matches in the sidebar and highlighting the matched row on its page.
- Groups and pages:
  - **General** (launch at login, sounds, widget position/visibility) · **Appearance** (macOS System Settings › Appearance patterns): *Appearance* Auto/Light/Dark as thumbnail pickers; *Widget size* Small / Regular (default) / Large as thumbnails of the real idle widget (same mic glyph as the widget — never a variant icon); *Ink* colour swatch row (SpeakType orange default, red, amber, green, blue, purple, white) with the selected name captioned under the row, *Line thickness* segmented Thin/Regular/Bold; *Widget position* pop-up menu with mini position glyphs, check on the current item, dividers, "Where I last dragged it" and **Reset widget position** (existing, `Settings.svelte:603`); *Show the widget* radio list (Always / Only while dictating or in a session / Never — menu bar only)
  - **Dictation** (hotkey, language, speech model as Fast · Balanced · Accurate, after-paste key, microphone + test)
    - **Speech models** — plain names over the *existing* catalog (`get_models` in `commands.rs`); the `.en` vs multilingual variant follows the Language setting (Automatic → multilingual, English → `.en`): **Fast** = `base` (~150 MB), **Balanced** = `small` (~500 MB, *Recommended*, matching the catalog's current recommendation), **Accurate** = `large-v3-turbo` (~1.5 GB, "fast + accurate"). **More models**: Most accurate = `large-v3` (~3 GB, memory note), Medium = `medium` (~1.5 GB, 16–24 GB Macs), Tiny = `tiny` (~75 MB). Sizes shown with the catalog's approximate labels; exact bytes come from `total_bytes` during download.
    - **Row states map 1:1 to the existing `model:progress` phases** (`downloader.rs`, `download_queue.rs`): `waiting` → "Waiting · starts after <model>" (queue runs 2 at a time; cancel); `downloading` → bar + "580 MB of 1.5 GB · about 2 min left" from `bytes_downloaded`/`total_bytes`/`eta_secs` (show the ETA only once it's stable, ≥ 4 s of samples, else just bytes); `retrying` → yellow dot + the existing message "Connection lost. Retrying in 8 s (2 of 5)"; `paused` → "Paused at 41%" + Resume (partial kept on disk); `error` (after 5 attempts) → red dot + plain reason + Try again; `done` → Downloaded; `loading` → "Getting ready…" while the model loads after selection; active model → In use.
    - **Layout (owner review, modelled on macOS Tahoe's Text Size sheet):** the Dictation page shows one *Speech model* row (current model, status, size) + **Manage models…**, plus a slim progress row only while something downloads. **Manage models…** opens the *Speech models* sheet: a **Faster ↔ More accurate slider** with one detent per model (labelled Fast / Balanced / Accurate) and a live line under it ("Balanced is in use · about 0.6 s to write out 10 s of speech on this Mac · ~500 MB" — the speed figure comes from a quick local benchmark of the downloaded model, otherwise omitted); below it **every model as a row** (tier-coloured icon, plain name, real model name + trade-off + size, trailing state control: Download / ring progress with bytes + time + pause / Paused · Resume / retrying message / Downloaded ⌃⌄ menu with Use and Delete / In use). Moving the slider to a model that isn't downloaded starts its download. Footer: disk used + Done.
    - Picking a model that isn't downloaded enqueues it with `load_after` and keeps the current one in use; SpeakType switches automatically when ready ("Keep working while it downloads").
    - Microphone row has a live input level meter next to the device picker. · **Sessions** (session hotkey, draw key, screenshot hotkey, capture copied text, live captions, keep audio, sessions folder, plus the Screen Recording row in context)
  - **One speech-model picker everywhere** (Try it, widget right-click → Speech model, History → Try with another model, Dictation row): a single grouped menu — *On this Mac* (downloaded Whisper models with state, plus "More Whisper models…" → Speech models sheet), *Your server* (the configured whisper.cpp server, or "Connect a whisper.cpp server…"), *Cloud* (models of configured services, or "Add a cloud service…"). Every entry point reuses the **same** pieces, so behaviour is identical everywhere: choosing a not-downloaded model starts its download from the menu itself (progress then shows in place — the onboarding footer, the Dictation row, or the sheet); "More Whisper models…" opens the **Speech models sheet** over whatever window you're in (onboarding included); "Connect a whisper.cpp server…" / "Add a cloud service…" open **Providers** (from onboarding: as a sheet over the onboarding window, returning to Try it when done). Exactly **one source is active at a time**; the only automatic switch is the existing fallback (service unreachable → On this Mac). No user-managed fallback chains at launch.
  - **Providers** (new page, sidebar group with Dictation and Sessions). **Speech to text** is chosen with three selectable cards (icon, title, one line; selected = blue ring + check), and the chosen provider's settings appear below as a normal grouped list (label left, control right; *On this Mac* shows none, *Your own server* shows Address + Test). Options: *On this Mac* (default, recommended), *Your own server* (whisper.cpp-compatible URL + Check — exists today as `api_url` / `check_server`), *Cloud service* (OpenAI, Groq, Deepgram, Custom OpenAI-compatible): set up **in dependency order, numbered 1–4**: (1) **Service** (pre-fills the base address for known services); (2) **Address** (editable, only needed for Custom); (3) **API key** (Keychain; secure field with an eye / eye-slash toggle at the trailing edge — revealing reads the Keychain item, which prompts for Touch ID or the Mac password; it re-masks after 30 s or when the window loses focus; paste works while masked) + **Connect**, which validates the key and fetches the model list (`GET {base}/models` on OpenAI-compatible APIs, filtered to speech-to-text models; known services also ship a curated fallback list); (4) **Model**, a searchable combobox of the fetched models — disabled with "Connect to load models" until step 3 succeeds, spinner while fetching, inline error on failure. Then **Test** (sends a bundled 3 s clip, reports latency). Amber disclosure row: "Your voice will be sent to <service>" (audio only; history/screens stay local); switch *If the service can't be reached, use On this Mac*. While a cloud provider is active the widget and menu-bar icon show a small cloud badge. **AI writing** (Pro, later): Off / On this Mac (llama.cpp — exists as `llm_server.rs`) / OpenAI-compatible (exists as `scribe_llm_*`).
    - Code gaps this closes (**new work**, `transcribe.rs:212-275`): no `Authorization` header is ever sent for transcription, so cloud APIs can't work today; the OpenAI-compatible path sends the *local* model name (`settings.model`, e.g. `medium`) instead of the provider's model; provider detection is URL substring matching (`is_openai_api`, `transcribe.rs:38-48`) — replace with an explicit provider setting. Move `scribe_llm_api_key` (plain settings JSON today) into the Keychain too.
  - **Permissions** (Microphone, Accessibility, Screen Recording: status + an inline **test** for each — Microphone: live level "Hearing you on <device>"; Accessibility: **Test typing** types "SpeakType test" into an inline field and reports success/failure; Screen Recording: status + "Open System Settings…". Plus **Reset permissions…** for when macOS forgets them after an update; it re-runs the relevant onboarding steps). Test states: each row is idle by default (status + "Test"); Microphone *Test* → live level + "Say something…" + *Stop test*; Accessibility *Test typing* → types into an inline 2-line field, shows "Typed successfully" (or the failure reason) + *Done*; Screen Recording has no test until allowed. Only one test runs at a time. · **Storage** (history retention, disk used by sessions/models, clear) · **Advanced** (transcription server, local LLM / endpoint for later features, reset)
- **App row → About**: version, Check for updates, licenses, Pro placeholder.
- Debugging lives in History → Details (not Settings).

### 6.8 Pro surfaces (inert at launch)
`ProBadge` component + `features.ts` flag map (all `true` at launch). Planned Pro: cleanup modes, custom vocabulary, unlimited session history/export formats, direct-to-agent send. Placement reserved in Settings (Dictation → Cleanup, Vocabulary) and Result view (Send to agent).

### 6.9 Quality bar (every screen)
- Dark + light; follows system by default.
- Loading, empty, error states designed (no blank panes, no raw error strings).
- Full keyboard operation, visible focus ring (see No glows rule), VoiceOver labels, `prefers-reduced-motion` respected, WCAG AA contrast.
- **Sheets (macOS Tahoe style, per the owner's reference screenshot):** a floating rounded panel (28 pt radius on all corners), horizontally centred, ~64–84 pt from the window top, over the whole window dimmed evenly (≈ 40% black); one shared inner inset (28–32 pt) for header, body and footer; primary button bottom-right; Esc = Cancel/Done. Used by Report a problem, Speech models, confirmation flows.
- **No glows** — with one deliberate exception (owner review): the floating **widget/HUD pill signals its state with a soft status halo**: `0 0 28px <state>/.34, 0 10px 26px rgba(0,0,0,.16)` (soft, blurred, **no hairline or hard edge at all**; the warning halo uses a warm brown-amber `rgb(176,104,36)/.5` rather than raw orange) in red (recording), blue (getting ready/active), amber #FF9F0A (warning, e.g. no speech), green (pasted/success). Nowhere else: no blurred coloured halos or coloured shadows on windows, cards, buttons, focus or popovers. Focus = macOS-style ring hugging the control: 1.5 pt, no gap, accent blue (`0 0 0 1.5px rgba(10,132,255,.85)`); orange is reserved for ink and the "live" indicator; state emphasis = solid 1 px border in the state colour; elevation = short neutral shadows only. **Popover/menu elevation token** (`--elevation-popover`): `0 0 0 .5px rgba(255,255,255,.14), 0 0 0 1px rgba(0,0,0,.55), 0 4px 12px rgba(0,0,0,.28)` — a crisp edge plus a short shadow; never a large blurred drop shadow, which makes straight edges look bowed.
- Motion: 120–200 ms ease-out for UI, 250 ms for ink fade; no layout jank.
- Copy: sentence case, plain words, no jargon on default paths.
- **Truncation and tooltips:** text that doesn't fit ends in "…" (middle-truncate file paths, tail-truncate everything else) and shows its full value in a tooltip after ~0.5 s hover — **only when it is actually truncated** (measure `scrollWidth > clientWidth`); text that fits never gets a tooltip. Icon-only buttons always have a tooltip with their name and shortcut.

### 6.10 Website (`apps/site`)
Single landing page + privacy + changelog + download redirect to latest DMG. Sections: hero (*Say it. Show it. Send it.* + looping product video), the three moments (Dictate · Show & Tell · Send to your agent), privacy (local, no account), who it's for (dev / QA / writers / notes), FAQ, download. Reuses tokens; static build; hosting is builder's call (GitHub Pages or Vercel).

## 7. Architecture

### 7.1 Windows (Tauri labels)
| Label | Kind | Notes |
|---|---|---|
| `main` | widget/HUD | existing; resizes between pill sizes; NSPanel |
| `overlay-<displayId>` | new | full-display, transparent, NSPanel, level above normal windows, `canJoinAllSpaces | fullScreenAuxiliary`, ignores mouse by default; created on session start, destroyed on stop |
| `review` | new | quick review panel, NSPanel attached to HUD, non-activating where possible |
| `settings` | existing, becomes the **main SpeakType window** | sidebar routes History + all settings pages (`settings.html` with a client-side router); `about` folds into the app row |
| `onboarding` | existing | rebuilt |

### 7.2 Rust modules (`apps/widget/src-tauri/src/`)
| Module | Responsibility |
|---|---|
| `session/mod.rs` | Session engine state machine (Idle/Active/Stopping), owns narration, marks, clips; emits events. Replaces orchestration bits of `scribe.rs` |
| `session/store.rs` | SQLite (evolve `scribe.rs` store): migrations via `PRAGMA user_version` |
| `session/assemble.rs` | Deterministic prompt assembly for each target (pure, unit-tested) |
| `session/bundle.rs` | Materialise bundle folder, image resizing |
| `overlay.rs` | Create/destroy per-display overlay windows, set panel level/behaviour, toggle `ignoresMouseEvents` |
| `input_tap.rs` | CGEventTap (listen-only) for modifier hold (flagsChanged) → `overlay:draw` on/off |
| `capture.rs` | ScreenCaptureKit freeze-frame (excludes all SpeakType windows), window-under-point lookup, ink compositing + crop/downscale; region capture via `screencapture -i` |
| `narration.rs` | Continuous mic capture + VAD segmentation → transcribe queue (reuse `audio.rs`, `chunk_session.rs`, `transcribe.rs`) |
| `clipboard.rs`, `ocr.rs`, `docx_export.rs` | existing; OCR only feeds FTS |
| `updater` | `tauri-plugin-updater`, GitHub Releases manifest, signed |

`llm.rs`, `llm_server.rs`, `templates.rs` stay but are out of the prompt path; templates become short prefaces.

### 7.3 Data model (migration v2)
`scribe_events` gains: `kind` (`speech|mark|screenshot|clip|note`), `t_start_ms`, `t_end_ms`, `ord REAL` (for reorder without renumbering), `image_path`, `caption`, `deleted INTEGER DEFAULT 0`, `payload TEXT` (JSON: ink bounds, source app, etc.). `scribe_sessions` gains `template`, `target`, `preface`, `bundle_path`, `ended_at`. FTS covers speech text, clip text, OCR text, caption.

### 7.4 IPC contract (single source of truth: `webui/src/lib/ipc.ts`; Rust serde `camelCase`)
Commands:
`session_start(template?) -> Session` · `session_stop() -> Session` · `session_active() -> Session|null` · `session_set_mute(bool)` · `session_list(query?) -> SessionSummary[]` · `session_get(id) -> SessionDetail` · `session_rename(id,name)` · `session_delete(id)` · `item_update(id, {text?, caption?})` · `item_delete(ids[])` · `item_move(id, ord)` · `item_split(id, charIndex)` · `prompt_render(sessionId, target) -> RenderedPrompt` · `prompt_copy(sessionId, target)` · `images_copy(sessionId)` · `bundle_reveal(sessionId)` · `export(sessionId, 'md'|'docx') -> path` · `overlay_pen(bool)` · `overlay_mark_complete(displayId, bounds)` (overlay → Rust).
Events:
`session:state` · `session:item_added` · `session:item_updated` · `narration:level` · `overlay:draw {active}` · `overlay:clear` · `capture:done` · `permissions:changed`.
The Phase-0 contract task writes these types + stubbed Rust commands first so frontend and backend agents work in parallel.

### 7.5 Frontend structure (`apps/widget/webui/src/`)
- **Source of truth for every control and state: the "Components" board** (buttons: primary/secondary/plain/destructive × default/hover/pressed/focus/disabled/busy, 32 and 28 pt; text fields: empty/focused/error/search/disabled/multi-line; Select: closed/open/disabled with check, highlighted, disabled-Pro item and separator; **Combobox** (type-to-filter Select for long lists — Language, Microphone): closed, open-filtered with bold matches and highlighted option, no-results message; **Checkbox** (multi-select, e.g. report contents, bulk delete): checked, unchecked, mixed, disabled, focus; **Thumbnail picker** (selected = 3 pt blue ring + bold label, others muted label); **Swatch row** (28 pt circles, selected = 2 pt gap + blue ring, name captioned below); **Pop-up menu with leading icons** (check column, icon/colour dot, label; dividers; trailing action item like "Choose…"/"Reset…"; every label — including action items with no icon — starts on one shared text column; **macOS pop-up placement: the menu opens with the currently selected item exactly over the button and its label on the button's text baseline/left edge**, not dropped below like a web dropdown; only Combobox/search menus drop below their field); **Radio list** for 2–4 short exclusive options; segmented, switch, radio; shortcut field idle/recording/conflict; status dots, Pro tag, progress; toast and widget halo). F2 builds `ui-gallery` to match that board one-to-one. **Button behaviour:** idle → hover (one step lighter, 120 ms ease) → pressed (one step darker, no movement) → focus ring for keyboard focus only (`:focus-visible`) → disabled (40% opacity, no hover/press response, label still readable). Cursor stays the arrow, per macOS convention (no hand cursor on buttons). Applies to primary, secondary, plain and destructive alike. **Destructive (macOS HIG):** a normal capsule with a red label — never a red fill, never the default button; label "Delete" when something is destroyed, "Remove" only when it leaves a list but still exists. Irreversible actions (delete session, clear history, reset permissions) confirm in an alert with Cancel as the default (Return) and Delete in red; undoable ones (delete a mark/clip) act immediately with an Undo toast and no alert. Keyboard shortcuts inside buttons are drawn as small keycaps (one per key, vertically centred), not inline glyph text. Glyphs with poor font metrics (↩ ⌫ ⇥ ⎋) are drawn as SVG icons inside the keycap so they centre exactly; modifier glyphs (⌃ ⌥ ⇧ ⌘) may use the system font.
- `lib/ui/` — component library: Button, IconButton, Input, Textarea, Select, Switch, Segmented, Tabs, Card, ListRow, Badge, ProBadge, Kbd, Tooltip, Popover, Dialog, Toast, Banner, EmptyState, Skeleton, Progress, Spinner. Each with both themes + focus states.
- `lib/theme.css` — add type scale, motion, z-index, overlay ink colours.
- `ui-gallery.html` (dev only) — every component in every state, both themes; used by the screenshot pipeline for review.
- `overlay/`, `history/`, `review/` — new entry points; `settings/` restructured; `settings.css` shrinks as screens migrate to `lib/ui`.

### 7.6a Software update experience (design: "Software updates" board)
- **Background by default:** check daily, download and verify silently (`tauri-plugin-updater`, signed manifest). No pop-up while checking or downloading.
- **Where it shows:** the sidebar app row ("Version 1.0 · Up to date" → "Downloading 1.1" with a thin bar → "Update ready · 1.1" + **Restart** and a blue dot on the app icon); the menu-bar icon gets a small blue dot and the menu's first item becomes **Restart to update to 1.1**. The floating widget never shows update UI.
- **The sheet** ("SpeakType 1.1 is ready", what's new, full release notes link, *Install updates automatically* checkbox, **Later** / **Restart now**) opens only when the user clicks Restart / the menu item, or once at launch after an update has waited 3 days — **never during dictation or a session**; during a session the primary becomes *Restart after this session*.
- **About › Software Update:** version + state with Restart, *Check for updates automatically*, *Download and install automatically*, *Last checked* + **Check now**; failures inline ("Couldn't check. You're offline." Try again; "Download failed" Retry).

### 7.6 Shipping
Developer-ID signing + notarization (existing notary scripts), `tauri-plugin-updater` with signed `latest.json` on GitHub Releases, DMG layout (existing `tools/dmg`). Panics logged locally; "Copy diagnostics" in Advanced. No telemetry.

## 8. Phases & workstreams (for parallel agents)

Each workstream owns its files; cross-cutting changes go through the contract in §7.4.

| Phase | Workstream | Owns | Depends on |
|---|---|---|---|
| **0 Foundation** | F1 Land current Scribe work (commit, green build/tests) | branch | — |
| | F2 Tokens + `lib/ui` library + `ui-gallery` | `lib/ui/*`, `theme.css` | F1 |
| | F3 IPC contract + Rust stubs + DB migration v2 | `ipc.ts`, `session/*`, `store.rs` | F1 |
| **1 Core loop** | D1 Widget/HUD polish + paste recovery | `widget/*`, `paste.rs` | F2 |
| | D2 Onboarding rebuild | `onboarding/*`, `permissions.rs` | F2 |
| | D3 Settings restructure | `settings/*` | F2 |
| **2 Show & Tell** | S1 Overlay windows + draw-key tap + ink canvas | `overlay.rs`, `input_tap.rs`, `overlay/*` | F3 |
| | S2 Narration engine + capture | `narration.rs`, `capture.rs`, `session/mod.rs` | F3 |
| | S3 HUD session mode | `widget/*` (after D1) | D1, F3 |
| **3 Review & History** | L0 Quick review panel (HUD-attached) | `review/*` | F2, F3, S3 |
| | L1 History page (inside the main window), search, unified list, Details tab | `settings/history/*` | F2, F3, D3 |
| | L2 Timeline view + editing | `settings/history/timeline/*` | L1 |
| | L3 Assembly, Result view, bundle, copy/export | `assemble.rs`, `bundle.rs`, `settings/history/result/*` | F3, L1 |
| **4 Ship** | R1 Updater, signing, release pipeline | `tauri.conf.json`, `tools/*` | — |
| | R2 Website | `apps/site/*` | F2 (tokens) |
| **Later** | Pro licensing; cleanup modes; vocabulary; agent detection + ACP/opencode direct send | — | launch |

**Review gate (Claude):** every workstream PR includes screenshots (both themes) from the screenshot pipeline / ui-gallery, `npm run check`, `npm test`, `cargo clippy -D warnings`, `cargo test`; acceptance criteria from §6 checked explicitly.

## 9. Risks & verification spikes (do first in their phase)
1. **Draw-key tap permission:** does a listen-only CGEventTap for `flagsChanged` work with Accessibility alone, or need Input Monitoring? Spike in S1 day 1; adjust onboarding.
2. **Overlay over full-screen apps & Spaces:** confirm panel level + collection behaviour shows over full-screen apps and across Spaces; multi-display hot-plug.
3. **Clean capture + composite:** ScreenCaptureKit content filter excluding all SpeakType windows; freeze-frame latency at draw-key down must be < 50 ms (pre-warm an `SCStream`/`SCScreenshotManager` when a session starts); window-under-ink lookup via `SCShareableContent`.
4. **Click-through toggle latency:** `ignoresMouseEvents` flip must apply on key-down before the next mouse-down.
5. **Continuous narration accuracy/latency** with VAD segmentation on `base` on M1.

## 10. Decisions settled during review (Claude's calls)
- **Dictation hotkey** stays a **toggle** (press to start, press to stop) on the existing default `Super+Control` (⌃⌘). No migration for v0.1 users. README's "⌘+Option+L" is stale — fix in docs.
- **New installs: download Fast (`base`, ~150 MB) first** so the first dictation happens within ~60 s, then **automatically queue Balanced (`small`) with `load_after`** and switch when ready (uses the existing download queue). Existing installs keep their model (current default `medium` stays for them). `.en` vs multilingual follows the **Language** setting: if Language is *Automatic* (today's default `auto`, `settings.rs:172`) use the multilingual variant; if it's set to English use `.en`.
- **Draw-key tap is its own listen-only CGEventTap** (`input_tap.rs`). It can't piggyback on `hotkey.rs`: that tap only exists when the dictation hotkey is a modifier chord like `Super+Control` (other hotkeys use `tauri_plugin_global_shortcut`, `hotkey.rs:47-70`) and it's an active `Default` tap at HID level (`hotkey.rs:158-161`). Spike #1 stays: confirm a listen-only `flagsChanged` tap works under Accessibility alone and detects Right-⌥.
- **Scribe is unreleased (not in v0.1.0), so it may be reshaped freely.** F1 commits the current work as a checkpoint; F3 writes a clean v2 schema with `PRAGMA user_version = 2` (dev databases are dropped/recreated, no migration). The Scribe settings tab and LLM-template UI are removed from the default path; `llm.rs`/`llm_server.rs`/`templates.rs` stay compiled behind Advanced for the later cleanup-modes feature. ADR-002 is historical except its clipboard/OCR/capture notes.
- **No fixed date ("the festival" = the build push).** Cut order if time runs short, first cut first: `.docx` export → Timeline split → chat target / Copy images → website changelog page → reorder via drag (keep up/down). Never cut: overlay, narration, Prompt copy, onboarding, updater, signing.

## 11. Left to the builder
Exact pixel values beyond the designs; VAD implementation choice; image resize library; website hosting; internal module boundaries inside each workstream; test structure.

## 12. Reconciliation with the existing code (audit 2026-11-18)

Owner feedback: designs must be grounded in what's built. Every item below is **Keep** (exists, must survive the redesign), **Change** (exists, behaves differently after), or **New** (doesn't exist yet). File refs are relative to `apps/widget/src-tauri/src/` and `apps/widget/webui/src/`.

**Widget & dictation**
- Keep: terminal-aware paste (⌘⇧V in terminals, `paste.rs:168,359`), clipboard restore after 600 ms (`paste.rs:365-370`), leading space on paste (`commands.rs:510`), server status dot starting/ready/error (`Widget.svelte:717-736`), permission-warning dot (`:866`), onboarding beacon (`:851`), "You're all set" card (`:807-816`) — reuse it as the end of onboarding's Try it.
- Change: paste-failure UI (uses existing signal), right-click → context menu, error events normalised to `{kind,message}`, Scribe strip in the widget (`Widget.svelte:920-972`) is replaced by the session HUD.
- New: "Getting ready…" pill while the model server loads, Esc-to-cancel via temporary global shortcut, per-device level meters in the mic picker, mid-dictation fallback to built-in mic with notice (today a missing device silently falls back at start, `audio.rs:557-571`, and a mid-stream error is only logged), Sounds toggle.

**Sessions (replacing Scribe mode)**
- Today Scribe is a mode switch (`scribe_enabled`, `settings.rs:63`); the first capture auto-creates "Scribe session <date time>" (`scribe.rs:166-195`) and each dictation closes it (`commands.rs:583`). **Change:** explicit session start/stop via ⌥⌘K / HUD; dictations no longer close sessions; narration is continuous inside a session.
- Keep: clipboard capture rules (text only, skip > 8,000 chars / 500 lines, skip if equal to recent dictation, ignore 700 ms after own paste; `clipboard.rs:61,93-113`), screenshot hotkey ⌥⌘S (`Super+Option+S`, `settings.rs:131`) with `screencapture -i` (`capture.rs:80`), OCR into FTS (`commands.rs:1840`), the per-session screenshot cap (50, `settings.rs:70,143`) and its "limit reached" error — now shown as an HUD message.
- Data model: **extend, don't rename.** Existing columns already cover `template`, `ended_at` (`scribe.rs:87-89`), `image_path`, `source_app`, `metadata` (≈ payload) (`scribe.rs:95-98`); the type column is `event_type` (`transcript|clipboard|screenshot`). Migration v2 adds only `mark` to `event_type`'s values plus `t_start_ms`, `t_end_ms`, `ord`, `caption`, `deleted`, and `target`/`preface`/`bundle_path` on sessions. §7.3 is superseded by this line.
- Paths: session data stays in `<config>/scribe/<id>/` (`scribe.rs:130-136`); the user-facing bundle (prompt.md, images) is written to `~/Documents/SpeakType/Sessions/` next to today's exports in `~/Documents/SpeakType/Scribe` (`commands.rs:1961`), which becomes `~/Documents/SpeakType/Exports`.
- IPC: the `session_*` commands **replace** the existing `*_scribe_*` commands; update the registration test `every_frontend_command_is_registered` (`lib.rs:702`) in the same change. Existing events to keep or rename deliberately: `sidecar:*`, `server:*`, `scribe:*` (→ `session:*`), `onboarding:opened/finished`, `widget:highlight`, `theme:changed`, `mictest:level/stopped`; `permissions:changed` already exists as `sidecar:permissions` (1 s poll, `lib.rs:155-190`) — reuse it, don't add a second.

**History**
- Keep: reprocess an entry (`reprocess_history_entry`, `commands.rs:1312`; UI `HistoryTab.svelte:267`) — surfaced in History Details as **Try with another model ▾** (same grouped model menu as onboarding, including cloud models when configured; result shown side by side with the original), delete audio only, clear all, play audio when *Save recording audio* is on (default off, `settings.rs:102`).
- Change: history cap of 100 entries (`history.rs:8`) becomes a Storage setting (Keep: 100 / 1,000 / Forever).
- New: an **outcome** field per entry (pasted / copied / paste failed / sent to …) — needed for the result rows; doesn't exist today (`history.rs:11-22`).
- **Details tab for dictations keeps the full debug record** (`debug.rs:48-179`): chunks, decode passes (temperature, score, dropped segments), per-segment quality flags (no_speech, low_confidence, repetition, sound_effects, empty, blocked_phrase; `quality.rs:66-73`), audio slices, last 25 records (`debug.rs:23`). Session Details is the summary shown in the designs; dictation Details shows this richer list.

**Onboarding**
- Keep: 3 gated requirements with Finish disabled until all are met (`Onboarding.svelte:104`), 2 s status polling (`:203`), auto-open when requirements are unmet at startup (`lib.rs:198-211`) and on hotkey (`commands.rs:131-134`), and **all recovery paths**: mic "Quit SpeakType" relaunch, Accessibility stale-entry instructions with "Quit & Reopen", the "Grant to: <path>" line, dev-build hints (`Onboarding.svelte:106-138,264-304`). These also power Permissions → Reset permissions….
- Change: the model-picker step becomes a background download (Fast first, then Balanced) shown in the footer.

**Settings pages: items that must not be dropped**
- Dictation: **Paste into** (active window vs. original window, default active; `settings.rs:24-27`, `Settings.svelte:958-970`); **After pasting** key with presets + custom chain, **current default Enter** (`settings.rs:98-100`, `Settings.svelte:173-177,984`) — keep the default for existing users, decide the new-install default in the plan; hotkey presets F9/F8 + custom capture (`Settings.svelte:785-796`); language list with search filter; **Save recording audio** (`settings.rs:31`).
- Advanced: **Hallucination filter** with retries/thresholds (`settings.rs:37-52`), server stop/restart and API URL with Check (`Settings.svelte:803-830`).
- General: **Reset widget position** (`Settings.svelte:603`).
- Bug to fix while restructuring: saving a new dictation hotkey calls `unregister_all`, which also drops the screenshot (and future session) hotkey (`hotkey.rs:33`).
- New: Launch at login (no autostart plugin today), Sounds toggle.
