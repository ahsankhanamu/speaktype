# Phase 1 — Core Loop (new widget, onboarding, main window) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: use superpowers:subagent-driven-development or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax. Read the program plan first; its Global Constraints and Lean code rules apply to every task here. Read `docs/superpowers/briefs/_common.md` too.

**Goal:** SpeakType's everyday dictation loop runs entirely on the new design. That covers:

- the widget, with every state on the Widget board;
- the 6-step onboarding;
- the single SpeakType window, with sidebar, search and every settings page, plus dictation History.

All of it is built from `@speaktype/ui`, `lib/data` and `bindings.ts`, with no legacy CSS left.

**Architecture:**

- **Rust:** two small workstreams add the settings fields, notices and commands the screens need. The contracts are fixed below, so the UI work runs in parallel against them.
- **Shared app components:** one UI workstream builds the composites used in several places (model menu, Speech models sheet, mic picker), plus the IPC mock harness for screenshots.
- **Surfaces:** three UI workstreams build the widget, onboarding and the main window on top of that.

**Spec:** `docs/superpowers/specs/2026-11-18-speaktype-launch-design.md`, §5, §6.1, §6.6, §6.7, §6.9 and §12. **Boards (2× PNG):** `docs/design/reference/`:

- `Widget.png`, `System.png`
- `OnboardingSteps.png`, `OnboardingModel.png`, `Onboarding.png`
- `Settings*.png`, `ModelsSheet.png`
- `Prompt.png`, used only for the History list styling
- `*-light.png`

## Waves and ownership

| WS | Model | Tasks | Owns (may edit) |
|---|---|---|---|
| **P1-R1** shell and settings | DeepSeek-V4.1-Flash | 1 | `src-tauri/src/settings.rs`, `src-tauri/src/lib.rs` (permission poll and plugin registration), `src-tauri/tauri.conf.json` (CSP only), `src-tauri/Cargo.toml`, `src-tauri/capabilities/*`, `bindings.ts` |
| **P1-R2** dictation notices and mics | GLM-5.3 | 2 | `src-tauri/src/{audio,commands,paste,history,hotkey}.rs`, new `src-tauri/src/notice.rs`, `bindings.ts` |
| **P1-U1** shared app components and harness | Kimi-K3.1-Pro | 3 | `webui/src/lib/app/**`, `webui/tests/visual/**`, `webui/playwright.config.ts`, `webui/package.json` |
| **P1-U2** widget | Kimi-K3.1-Pro | 4 | `webui/src/widget/**` |
| **P1-U3** onboarding | Kimi-K3.1-Pro | 5 | `webui/src/onboarding/**`, `webui/onboarding.html` |
| **P1-U4** main window | Kimi-K3.1-Pro | 6 | `webui/src/settings/**` (renamed to `webui/src/app-window/**`), `webui/settings.html`, `webui/src/about/**` (folded in), `webui/src/lib/settings.css` (deleted), `webui/src/lib/ModelGrid.svelte` / `StSelect.svelte` / `InfoPopover.svelte` / `ThemeTabs.svelte` / `SettingsFooter.svelte` (deleted) |

**Order:**

1. **Wave 1:** R1, R2 and U1 run in parallel, and start only after F5 (data layer) and F4b (provider wiring) have merged. U1 codes against the contracts below, using the mock harness.
2. **Wave 2:** U2, U3 and U4 run in parallel after U1 merges. R1 and R2 must also be merged before their screens' final screenshot review.

`bindings.ts` is generated, so R1 and R2 both regenerate it. Whichever merges second regenerates it again, and that resolves the conflict.

## Fixed contracts (both sides build to these)

**Settings fields added by R1** (`settings.rs`, serde snake_case like the existing struct; each value is the default when missing):

| Field | Type | Default |
|---|---|---|
| `sounds` | `bool` | `true` |
| `widget_size` | `"small" \| "regular" \| "large"` | `"regular"` |
| `widget_visibility` | `"always" \| "active" \| "never"` | `"always"` |
| `ink_color` | `String`: `"orange" \| "red" \| "amber" \| "green" \| "blue" \| "purple" \| "white"` | `"orange"` |
| `ink_thickness` | `"thin" \| "regular" \| "bold"` | `"regular"` |
| `draw_key` | `"right_option" \| "right_command" \| "fn"` | `"right_option"` |
| `draw_anytime` | `bool` | `true` (pending the owner's answer on always-on drawing; Phase 2 reads it) |
| `history_limit` | `Option<u32>`; `None` = forever | `Some(100)` |
| `launch_at_login` | `bool` | `false` |
| `onboarding_step` | `u8`: last step reached, for resume | `0` |

- **New installs only:** `post_paste_keys = None` (Nothing). Existing settings files keep their value. A file that has no `version` key is an existing install; R1 adds `version: 2`.

**Commands added by R1:**

- `set_launch_at_login(on: bool) -> Result<(), IpcError>`, via `tauri-plugin-autostart`.

**Notices and commands added by R2:**

- **Event** `DictationState { phase: "idle" | "preparing" | "recording" | "transcribing" | "done" | "error", notice: Option<Notice> }`.
- **Type** `Notice { kind: NoticeKind, message: String }`.
- **`NoticeKind`:** `pasted`, `copiedNotPasted`, `noSpeech`, `busy`, `longAudio`, `serverError`, `micUnavailable`, `micFallback`, `providerFallback`, `screenshotLimit`.
- **What it replaces:** every `sidecar:*` status or error event that the widget reads today, which are `recording`, `transcribing`, `result`, `error`, `no_speech`, `busy`, `long_audio` and `server:*` status. Each of those keeps firing until U2 merges, and is then deleted in U2's PR. The R2 report lists them.
- **Copy, fixed by the spec:**

  | Kind | Message |
  |---|---|
  | noSpeech | "Didn't catch that" |
  | busy | "Still working on the last one" |
  | copiedNotPasted | "Copied. Press ⌘V to paste" |
  | serverError | "Speech model couldn't start" |
  | longAudio | "Long recording, this may take a moment" |
  | micUnavailable | "Microphone unavailable" |
  | micFallback | "Switched to MacBook microphone" |

- **`preparing`** is emitted when the hotkey fires while the model server is loading. That is the "Getting ready…" state.
- **Commands:**
  - `paste_last() -> Result<(), IpcError>`: re-pastes the newest history text.
  - `mic_levels_start() -> Result<(), IpcError>` and `mic_levels_stop()`.
  - While started, the event `MicLevels { levels: Vec<{ device: String, level: f32 /* 0..1 RMS, smoothed */ }> }` fires every 50 ms, one short-lived input stream per device. They stop automatically after 30 s with no restart.
- **History** entries gain `outcome: "pasted" | "copied" | "pasteFailed"`, with `None` for old entries.
- **Esc during recording** cancels. A temporary global Esc shortcut is registered on `recording` and removed on any other phase.
- **Mid-dictation mic loss** falls back to the built-in mic, keeps recording, and emits `micFallback`.

## Review Focus (Phase 1)

1. **Permission granted while running.** Accessibility granted after launch must arm the dictation hotkey without a restart (bug found 2026-11-19: `hotkey.rs` never retries). Test in R1.
2. **Hotkey pressed while the model loads.** The pill shows "Getting ready…" at once, and recording starts by itself when ready, with no lost press. Test in R2 (state sequence) and U2 (render).
3. **Mic picker open for a long time, or the window hidden.** The per-device streams stop after 30 s or on close, and no audio stream outlives the picker. Test in R2 (auto-stop) and U1 (calls stop on destroy).
4. **Settings window open while the widget changes a setting** (mic or model from the right-click menu). The window updates without reopening. Test in U4, via `liveQuery` invalidated by `settings-changed`, which R1 emits from `save_settings`.
5. **Tiny and huge content.** A 0-byte model size, 300-character mic names, and 1,000 history entries. Rows truncate with tooltips only when truncated, and the History list stays smooth (virtualized). Tests in U1 and U4.

---

### Task 1 (P1-R1): settings v2, launch at login, shell fixes

**Files:** `settings.rs`, `lib.rs`, `tauri.conf.json`, `Cargo.toml`, `capabilities/default.json`, `bindings.ts`.

- [ ] **Failing tests:**
  - `settings_defaults_for_new_install`: no file gives `post_paste_keys == None`, `version == 2`, and every default in the table.
  - `settings_existing_install_keeps_post_paste`: a file without `version` that has `post_paste_keys: "enter"` keeps `"enter"`.
  - `settings_roundtrip_new_fields`.
  - `save_settings_emits_settings_changed`: the `SettingsChanged {}` typed event fires after the write.
- [ ] **Implement** the fields, the `SettingsChanged` event and `set_launch_at_login` (`tauri-plugin-autostart`, LaunchAgent).
- [ ] **CSP:** add `ipc: http://ipc.localhost` to `connect-src` in `tauri.conf.json`. Verify by launching `make dev`: the log must show zero "IPC custom protocol failed" lines. Record the count in the report.
- [ ] **Hotkey re-arm:** in the 1 s permission poll (`lib.rs:96-110`), when Accessibility flips false → true, call the existing hotkey registration once. Test the flip detection as a pure function `fn accessibility_became_granted(prev: bool, now: bool) -> bool`, and check the real behaviour with `make dev`: grant the permission while running, then dictate. Note the result in the report.
- [ ] Regenerate bindings, run `make verify`, commit `feat(app): settings v2, launch at login, CSP ipc, hotkey re-arm`.

### Task 2 (P1-R2): dictation state, notices, mic levels, Esc, fallback, outcome

**Files:** `notice.rs` (new), `commands.rs`, `audio.rs`, `paste.rs`, `history.rs`, `hotkey.rs`, `bindings.ts`.

- [ ] **Failing tests:**
  - `notice_copy_matches_spec`: each kind maps to its exact message.
  - `dictation_emits_preparing_then_recording_when_server_loading`: inject a fake server-ready signal. The phases come out as `preparing → recording`, and the press is not lost.
  - `paste_failure_records_outcome_and_notice`: `paste_text` returning false gives `outcome = pasteFailed` and the notice `copiedNotPasted`.
  - `mic_levels_auto_stop_after_30s`: with tokio paused time, no stream remains after 30 s.
  - `esc_shortcut_registered_only_while_recording`: registration calls are recorded for the phase sequence idle → recording → transcribing.
- [ ] **Implement.** Per-device streams reuse the `LevelMonitor` code in `audio.rs:196-283`. Fallback rules:
  - On a stream error mid-recording, re-open the default built-in input and continue appending samples.
  - Emit `micFallback` once.
- [ ] Regenerate bindings, run `make verify`, commit `feat(dictation): typed state + notices, mic levels, Esc cancel, mic fallback, outcome`.

### Task 3 (P1-U1): app components and the screenshot harness

**Files:**
- `webui/src/lib/app/models.ts` and `models.test.ts`
- `ModelMenu.svelte`, `SpeechModelsSheet.svelte`, `MicPicker.svelte`
- `webui/tests/visual/{mock-ipc.ts, fixtures/*.json, harness.spec.ts}`
- `webui/playwright.config.ts`

**Interfaces (produces):**

```ts
// models.ts — pure, all copy from spec §6.6/§6.7
export type Tier = 'fast' | 'balanced' | 'accurate' | 'more';
export function plainModel(id: string): { name: string; tier: Tier; detail: string; size: string }; // base→Fast ~150 MB, small→Balanced ~500 MB Recommended, large-v3-turbo→Accurate ~1.5 GB, large-v3→Most accurate ~3 GB, medium→Medium ~1.5 GB, tiny→Tiny ~75 MB; .en variants map to the same names
export function modelState(m: ModelInfo, progress: ModelProgress | undefined, activeId: string): { label: string; tone: 'success' | 'accent' | 'warning' | 'danger' | 'neutral'; action: 'use' | 'download' | 'cancel' | 'resume' | 'retry' | null };
//   labels: "In use" | "Downloaded" | "Downloading · N%" | "Waiting · starts after <model>" | "Paused at N%" | "<existing retrying message>" | "Download · <size>" | "Getting ready…"; ETA shown only after ≥ 4 s of samples
```

- **`ModelMenu`:**
  - Props: `{ trigger: 'row' | 'compact' }`.
  - Groups: On this Mac, then Your server, then Cloud, then the actions.
  - "More Whisper models…" opens the Speech models sheet. "Connect a whisper.cpp server…" and "Add a cloud service…" call the prop `onopenproviders`.
  - Choosing a model that isn't downloaded calls `queueModelDownload(id, loadAfter=true)`.
  - Data comes from `liveQuery('models', getModels, { invalidateOn: ['model:progress', 'settings-changed'] })`.
- **`SpeechModelsSheet`:** a `Sheet`. It holds:
  - a Faster ↔ More accurate slider (Fast / Balanced / Accurate detents);
  - the live line;
  - a row per model, with its state control;
  - a footer with the disk space used and Done.

  Use the ModelsSheet board. Leave out the benchmark speed figure; that is new work for later.
- **`MicPicker`:**
  - Shows System default plus each device, with its own live level from the `mic-levels` event.
  - The selected device has a check. Disconnected devices read "Not connected". Bluetooth devices get a quality note.
  - Calls `micLevelsStart` on mount and `micLevelsStop` on destroy.
- **The harness:** `mock-ipc.ts` installs `window.__TAURI_INTERNALS__`:
  - `invoke` resolves from `fixtures/<command>.json`;
  - `emit()` is exposed so a spec can fire events.

  Specs open `dist/*.html?fixture=<name>&theme=light|dark` from a static server and screenshot at 2×. This replaces the old `tools/assets/render_settings_screenshots.mjs` for review purposes; leave that script alone.

- [ ] **Failing tests:**
  - `models.test.ts`: the plain-name table, and every state label from the spec, including the ETA rule.
  - `MicPicker calls micLevelsStop on destroy`, with a mocked bindings module.
  - `ModelMenu groups and actions`.
  - `harness.spec.ts` renders the ModelMenu, sheet and picker in both themes.
- [ ] **Implement.** Compare against `ModelsSheet.png`, `OnboardingModel.png` and `SettingsDictation.png`, in two compare-and-fix rounds.
- [ ] Run `make verify`, commit `feat(webui): shared app components + IPC mock harness`.

### Task 4 (P1-U2): widget

**Files:** `webui/src/widget/**`, rewritten from `Widget.svelte`.

- [ ] **Failing tests**, one component test per Widget board state, driven by `DictationState` fixtures:
  - idle, recording (waveform, timer, ✕, stop), transcribing, "Getting ready…", noSpeech (fades after 2 s), pasted (0.5 s then idle);
  - copiedNotPasted, with Paste again calling `pasteLast`;
  - micUnavailable, with Choose mic opening `MicPicker` in a popover;
  - micFallback, serverError with Try again.
  - Also:
    - The halo tone per state.
    - `widget_size` scales: small 30, regular 40, large 50.
    - Right-click opens the context menu: Microphone ▸, Speech model ▸ (`ModelMenu`), History, Settings, Hide widget.
    - The "Finish setup" dot shows when onboarding is unfinished.
- [ ] **Implement.** Use `@speaktype/ui` `Halo`, `IconButton` and `ContextMenu`, and subscriptions only through `lib/data`. Delete every `sidecar:*` listener the R2 report names, and remove those emits from Rust in the same PR. Keep the drag-to-move, the server status dot and the permission-warning dot (spec §12).
- [ ] Take harness screenshots in both themes and compare them with `Widget.png`. The widget is dark in both themes.
- [ ] Run `make verify`, commit `feat(widget): rebuilt on @speaktype/ui with typed dictation state`.

### Task 5 (P1-U3): onboarding

**Files:** `webui/src/onboarding/**`, rewritten.

- [ ] **Failing tests:**
  - Step order: Welcome, then Microphone, then Accessibility, then Speech model, then Try it.
  - Labels read "Step N of 4" (Welcome is unnumbered).
  - Continue is disabled until the step is done. Microphone and Accessibility auto-advance on grant (`checkPermissions` poll every 2 s, which exists today).
  - Resume: it starts at `settings.onboarding_step`.
  - The step transition is `step({ direction })`, and a cross-fade when motion is reduced.
  - The Speech model slider calls `queueModelDownload` for the chosen tier with `loadAfter=true`, and Fast is queued first on a new install.
  - Try it shows the configured hotkey as keycaps, and the success state, then calls `finishOnboarding`.
  - Every recovery path in spec §12 is kept: mic "Quit SpeakType", the Accessibility stale-entry help with "Quit & Reopen", the "Grant to: <path>" line, and the dev-build hints.
- [ ] **Implement.** The window is 960 × 640, using the `OnboardingSteps`, `OnboardingModel` and `Onboarding` boards. `MicPicker` comes from U1, and `ModelMenu` is used inside Try it.
- [ ] Take screenshots of every step in both themes, run `make verify`, and commit `feat(onboarding): six-step first run`.

### Task 6 (P1-U4): the SpeakType window

**Files:**
- `webui/src/app-window/` (rename `settings/`):
  - `App.svelte`, `router.ts`, `search.ts`
  - `pages/{History,General,Appearance,Dictation,Sessions,Providers,Permissions,Storage,Advanced,About}.svelte`
- `settings.html` (entry kept; the window label stays `settings`)
- Delete the legacy files listed in the ownership table.

**Interfaces:**
- `router.ts`: hash routes `#/history`, `#/general` … `#/about`. It remembers the last page in `localStorage`, read inside try/catch.
- `search.ts`: `index: { page: string; label: string; keywords: string[]; anchor: string }[]` holds one entry per individual setting. `filter(q)` returns matches, the sidebar shows them, and choosing one scrolls to and highlights the row.

- [ ] **Failing tests:**
  - Every sidebar item routes.
  - Searching "draw" returns Sessions › Draw while holding.
  - The last page is remembered.
  - Each page renders every field from spec §6.7 and the §12 "must not be dropped" list, bound to the real settings field:
    - **Dictation:** Paste into, After pasting (presets plus custom chain), hotkey presets F9/F8 plus custom, the language combobox, Save recording audio, the Speech model row plus Manage models….
    - **Advanced:** the hallucination filter with its thresholds, server stop/restart, and the API URL with Check.
    - **General:** launch at login, sounds, Reset widget position.
  - **Providers:** the three cards (On this Mac, Your own server, Cloud service).
    - Cloud has four numbered steps in dependency order.
    - The key goes through F4b's `stt_set_api_key`, and the SecureField reveal through `stt_reveal_api_key`.
    - The Model combobox is disabled until Connect succeeds.
    - Test, plus the amber disclosure row.
  - **Permissions:** tests that are idle by default, one test at a time.
  - **History:** the dictations list grouped by day, virtualized. Each entry offers copy, re-paste, delete, play audio when present, and reprocess with `ModelMenu`, with its outcome shown. Session entries show as rows linking to a "coming soon" Timeline placeholder (Phase 3).
  - The settings window updates live on `settings-changed` (Review Focus 4).
- [ ] **Implement** on `@speaktype/ui` `Sidebar`, `Group`, `Row` and friends. The sidebar metrics come from tokens, and the app row opens About.
- [ ] Take screenshots of every page in both themes and compare against the Settings* boards, in two rounds. Run `make verify`, commit `feat(app-window): one SpeakType window on @speaktype/ui`.

---

## Phase 1 exit (Claude)

- [ ] Every workstream has been reviewed (code plus screenshots against the boards) and merged.
- [ ] `make verify` is green on `main`.
- [ ] I run `make dev` and complete a manual run-through:
  - onboarding on a fresh settings directory;
  - dictation, with paste success and paste failure;
  - the model switch and the "Getting ready…" state;
  - the mic picker levels;
  - every settings page in both themes.
- [ ] I ask the owner to dogfood the build, then write the Phase 2 plan, which starts with the spikes.
