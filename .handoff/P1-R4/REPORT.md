# P1-R4 — widget follow-ups in Rust · report

Owner model: DeepSeek-V4.1-Flash. Branch `ws/P1-R4`, worktree `/Users/ahsan/Desktop/speaktype-ws/P1-R4`.

Note on history: the branch was built against U2, then merged `main` (U4, `d6ddd68`/`eb64928`) on the
reviewer's instruction, so everything below is checked against the merged widget/app-window code.

## Tasks done

| # | Brief item | Commit |
|---|---|---|
| 1 | Old emits deleted (`sidecar:device_changed`, `sidecar:error`); pinned by a source-scanning test | `c763c52`, `39455ed` |
| 2 | `transcribe::emit_fallback` → typed `DictationState { Done, providerFallback }`, "Couldn't reach <service>. Used On this Mac." | `c763c52` |
| 3 | Mic fallback emits `DictationState { Recording, micFallback }` at the moment of switch; `Done` carries the paste outcome | `dfafaf7`, `c763c52` |
| 4 | `open_settings(section)` lands on `#/<section>`; tray `History…` item; widget History item | `51c46bc` (first cut on `ws/U3-settings`), merged in `81383de` |
| 5 | `tauri.conf.json` main window 92 × 92 | `1f5acfe` |
| 6 | Widget's dead `setInterval` lint exemption removed | `b91841e`, `f837a38` |

Follow-up commits: `8d121ff` (dead latch read removed, clippy), `56c34a4` (stale duplicate tests resolved
after the merge).

### 1. Old emits
Grepped `apps/widget/webui/src` on the merged tree. Still listened after U2/U4:
`sidecar:audio_level` (Recording), `sidecar:permissions` (widget, onboarding, Permissions page),
`sidecar:pasted` (DebugRecords — see Deviation 1), `mic-devices-changed` (MicPicker).
Nothing listens to ready / recording_* / transcribing / long_audio / no_speech / busy / error /
crashed / device_changed / onboarding_required — none of those are emitted by this branch (they were
already gone from the Rust in Phase 0/1; the only leftover was `sidecar:error` from `transcribe.rs`,
now removed). `lib.rs::ipc_surface::only_the_events_the_new_widget_listens_to_are_emitted` scans
`src-tauri/src/*.rs` for quoted event literals and fails on anything outside the kept set
(`sidecar:audio_level`, `sidecar:permissions`, `server:*`, `dictation:*`, `onboarding:*`,
`mic-devices-changed`, `widget:highlight`). A removal of `sidecar:error` makes it red (observed).

### 2–3. Provider and mic fallback
- `controller::provider_fallback(&ProviderConfig)` returns `Done` + `NoticeKind::ProviderFallback`
  with the service name in the message.
- `transcribe_pass`'s fallback closure calls `commands::emit_dictation` with that state; `stt_cmds::stt_test`
  passes a no-op (a Test is not a take).
- `FallbackMic` takes an `on_switch` callback fired once, inside `on_error`, at the switch; `begin_recording`
  passes a closure that emits `controller::mic_fallback()` (`Recording` phase). `standalone stop()` no longer emits.
- Front-end (`dictation.svelte.ts`): a `recording` state carrying `micFallback` renders as a notice with a
  `tone: 'recording'` halo (red) and stays until the take ends; the `done` notice then shows the pasted /
  copied outcome. Widget test: notice during recording, red halo, fit unchanged, still shown 10 s later,
  gone when `transcribing` arrives.

### 4. `open_settings(section: Option<String>)`
The branch's first cut was made before U4 merged; U4 replaced the six-tab Settings window with the
one SpeakType window and `app-window/router.svelte.ts` already routes on `#/<page>`. I reproduced the
same Rust change on `main` as branch `ws/U3-settings` (worktree `../.u3-settings`, merged as `81383de`):

```rust
fn main_window_url(window_url: &tauri::Url, section: Option<&str>) -> tauri::Url
const MAIN_SECTIONS: &[&str] = &["history","general","appearance","dictation","sessions","providers","permissions","storage","advanced","about"];
pub async fn open_settings(app: AppHandle, section: Option<String>) -> Result<(), String>
```
- an unknown section is an error ("Unknown section: …");
- an open window navigates to the section (and keeps its page when no section is given);
- the tray has `History…` between the record item and `Settings`, calling `open_settings(app, Some("history"))`
  and logging a failure instead of swallowing it;
- the widget's History item calls `commands.openSettings('history')`;
- `ipc.ts`'s `openSettings()` wrapper passes `null` so the other callers keep their behaviour;
- `bindings.ts` regenerated; `ipc_surface::open_settings_is_registered_with_its_section_argument` keeps it
  fresh, plus `commands::tests::open_settings_lands_on_the_section_it_is_given` and
  `open_settings_sections_are_the_windows_pages`.
- The History-listener changes: `MicPicker` follows `mic-devices-changed` instead of polling every 3 s.

### 5. Window size
`"width": 92, "height": 92` for `main`. Test `ipc_surface::the_widget_window_opens_at_the_idle_pill_plus_its_halo_margin`
reads `tauri.conf.json` (was 72 × 88 → red → green); `windowFit.test.ts` already pins 92 × 92 for a 44 pt pill.

### 6. ESLint
`src/widget/Widget.svelte` removed from the `no-restricted-syntax` exemption (the file still listed
`src/settings/Settings.svelte`, which no longer exists — removed too). `eslint src` exits 0 with the rule
enforced for the widget. `widget.test.ts › lint policy › the widget does not get a setInterval exemption`
fails when the entry is put back (observed).

## Commands run (tails)

```
$ npm ci && npm --prefix apps/widget/webui run build
✓ built in 4.42s
$ cp /Users/ahsan/Desktop/speaktype/apps/widget/src-tauri/binaries/* apps/widget/src-tauri/binaries/
whisper-server-aarch64-apple-darwin

$ cargo test -p speaktype --lib            # after every item
test result: ok. 100 passed; 0 failed; 1 ignored; ...

$ npx vitest run src/widget src/lib/ipc.test.ts
Tests  40 passed (40)
$ npx svelte-check --tsconfig ./tsconfig.json
svelte-check found 0 errors and 0 warnings
$ npx eslint src
(exit 0)

$ cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings
Finished `dev` profile ... (clean)

$ make verify     # log: /tmp/verify-P1-R4.log, final run reported below
```

`make verify` finding: `widget.spec.ts › {dark,light} › recording` occasionally differs by 40 px
(0.01%) in the 9th waveform bar — the test's waveform animates with `requestAnimationFrame` while the
page clock is frozen. Not caused by these items (the recording state is untouched); it passes 3/3 in
isolation and 5/6 under `--repeat-each=3`. Left as is.

## Deviations

1. **`DebugRecords.svelte` (app-window, owned by U4) still invalidated on `sidecar:pasted`**, which this
   branch stops emitting. One-line change to `invalidateOn: ['dictation-state']` — made inside the merge
   rather than the open-semantics choice, because the alternative is a debug list that never refreshes.
2. **`emit_dictation` split out of `remember_dictation`.** The mic fallback fires from the audio thread's
   stream-error callback; it needs to reach the pill without writing the stored state (the controller still
   holds `Recording` and will publish `transcribing` itself). `remember_dictation = store + emit_dictation`.
3. **`transcribe` / `transcribe_verified` / `transcribe_chunked` lost their `R: Runtime` parameter** while
   `emit_fallback` was being replaced, then were restored when the closures needed `AppHandle<R>`. Net: unchanged
   signatures.
4. **`FallbackMic::switched()` removed** (its only reader, `AudioRecorder::builtin_fallback`, is replaced by the
   switch callback); clippy `-D warnings` caught it.
5. **`MicPicker` poll → event.** Not in the brief text but the brief's item 1 names `mic-devices-changed` as
   the replacement for `sidecar:device_changed`; the picker's 3 s poll was what the event exists to avoid.
6. **Branch shape.** `ws/U3-settings` (one commit, `51c46bc`) exists only to carry item 4 onto U4; it is merged
   into `ws/P1-R4` and can be deleted with the worktree.
7. **The `settings/Settings.svelte` and `settings/MicTest.svelte` edits from the pre-merge work are gone** —
   U4 deleted those files; the new app-window already routes on `#/<page>`.

## New dependencies
None.

## Open questions
- None blocking. Reviewer to check on the real app: the mic-fallback pill (unplug a chosen USB/BT mic mid-take
  → "Switched to MacBook microphone" with the red halo, still recording), tray `History…`, widget right-click
  `History`, and the 92 × 92 opening window (no jump at first paint).
