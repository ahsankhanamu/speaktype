# P1-R2 — dictation state, notices, mic levels, Esc, fallback, outcome — Report

Workstream: P1-R2 (owner model: DeepSeek-V4.1-Flash) · Branch `ws/P1-R2`
Plan: `docs/superpowers/plans/2026-11-19-phase1-core-loop.md` Task 2 + Fixed contracts/Review Focus.
Spec: §6.1, §6.7 (row states), §12 (Widget & dictation, History, onboarding keep list).

## Tasks done

| Commit | What |
|---|---|
| `915655c` | Task 2 implementation (see below) |
| `828e5cd` | Merge `main` (`36834b7`…`8bbb514`): settings v2 + hotkey re-arm (R1), onboarding (U3), U1 components |
| `1b1b964` | Report committed |
| round 1 | Merge `main` (`4db7e7b`, the shared-target `CARGO_MANIFEST_DIR` test fix) + `fix(dictation): bound the getting-ready wait (review round 1)` — see below |

Task 2 as delivered:

- **Typed state** — `src-tauri/src/controller.rs`: `DictationPhase` (`idle/preparing/recording/transcribing/done/error`) and `DictationState { phase, notice }`, emitted as the typed `dictation-state` event. `preparing` is set the instant the hotkey fires while the embedded server is down; the press waits on `ServerGate` and **records by itself** when the model answers (no lost press).
- **Notices** — `src-tauri/src/notice.rs`: `NoticeKind`/`Notice` with the plan table's copy verbatim (`notice_copy_matches_spec`), plus `pasted`, `providerFallback`, `screenshotLimit` whose copy the plan leaves open (see Deviations).
- **Esc cancels** — `hotkey::EscShortcut` arms a global `Escape` on `recording` and releases it on every other phase; wired through `EscHolder` in `lib.rs`, driven by `remember_dictation`.
- **Mic levels** — `audio::DeviceLevels`: one short-lived metering stream per input device, `mic-levels` event every 50 ms with smoothed 0..1 RMS per device, released on `mic_levels_stop`, on window hide/exit, and automatically 30 s after `mic_levels_start`; **every `start` re-arms** the deadline (the picker's 20 s keep-alive, `mic_levels_start_resets_auto_stop`).
- **Mid-dictation mic loss** — `audio::FallbackMic`: a stream error opens the built-in input into the same take buffer, keeps recording, `micFallback` shown once per take.
- **History `outcome`** — `history::DictationOutcome` (`pasted | copied | pasteFailed`), `None` on old entries; `paste_failure_records_outcome_and_notice` path in `process_recording` (`paste_text` false ⇒ `pasteFailed` + `copiedNotPasted`).
- **Commands** — `paste_last`, `mic_levels_start`, `mic_levels_stop`, `delete_model` (refuses the active model, `IpcError{kind:'modelInUse'}`).
- **Retry copy** — `downloader::retry_message` now produces "Connection lost. Retrying in 8 s (2 of 5)" for the `retrying` phase.
- **Kept, not deleted** — every `sidecar:*`/`server:*` event U2 owns deletion of still fires (list below).

## What U2 deletes (the `sidecar:*` / `server:*` set R2 replaced)

Emitted today, each now superseded by `dictation-state`:

| Event | Replaced by |
|---|---|
| `sidecar:recording_started` / `sidecar:recording_stopped` / `sidecar:recording_cancelled` | `dictation-state.phase` (`recording`, `transcribing`, `idle` on cancel) |
| `sidecar:transcribing` | `dictation-state.phase = transcribing` |
| `sidecar:pasted` | `dictation-state.phase = done`, notice `pasted` |
| `sidecar:error` | `phase = error` + notice (`serverError`, `copiedNotPasted`, `micUnavailable`) |
| `sidecar:no_speech` | notice `noSpeech` |
| `sidecar:busy` | notice `busy` |
| `sidecar:long_audio` | new `dictation:long-audio` (a notice in `done`/`error` only would be wrong mid-transcribe; U2 can render it as the `longAudio` notice) |
| `server:error` (server start failure during dictation) | `phase = error`, notice `serverError` |
| `sidecar:audio_level` | unchanged (waveform), and `dictation:mic-fallback` is the new one-shot mic-switch signal |

`sidecar:permissions`, `sidecar:device_changed`, `onboarding:opened/finished`, `server:starting/ready` are not dictation-state signals and stay.

## Exact commands run (tails)

```bash
npm ci; npm --prefix apps/widget/webui run build; cp ../speaktype/.../binaries/*   # setup, "✓ built in 4.24s"
cargo test -p speaktype --lib audio::      # RED first (no DeviceLevels/smooth_levels), then GREEN: "ok. 5 passed"
cargo test -p speaktype --lib controller:: # GREEN: preparing→recording etc. in the suite: included below
cargo test -p speaktype                    # "ok. 80 passed; 0 failed; 1 ignored"
cargo test -p speaktype export_bindings_write -- --ignored   # regenerates bindings.ts, "ok"
cargo fmt --all --check                    # "FMT OK"
cargo clippy --workspace --all-targets -- -D warnings        # "CLIPPY OK" (no warnings)
cargo test --workspace                     # speaktype 80 ok; capture 3 ok; session 27 ok; assemble_golden FAILED (see below)
npm run -ws --if-present check             # "svelte-check found 0 errors and 0 warnings" (×2 workspaces)
npm run -ws --if-present lint              # clean (0 error lines)
npm run -ws --if-present test              # "Test Files 5 passed / Tests 56 passed", "Test Files 9 passed / Tests 95 passed"
npm -w @speaktype/ui run visual            # "33 passed (11.7s)"
make verify                                # round 0: EXIT 2 (shared-target test binary); round 1, after main's 4db7e7b: EXIT 0
```

### Round 0: `make verify` failed on a shared-target test binary (fixed on main by `4db7e7b`)

> **Resolved.** Main's `4db7e7b` resolves `CARGO_MANIFEST_DIR` at run time in `assemble_golden.rs` and `ipc_export.rs`. After merging it, `make verify` exits 0 (see Review round 1). The analysis below is kept for the record.

`make verify` stops at `cargo test --workspace` with:

```
Running tests/assemble_golden.rs (/Users/ahsan/Desktop/speaktype-ws/.target/debug/deps/assemble_golden-c9d708f4422b9b9c)
golden file: Os { code: 2, kind: NotFound, ... }
test result: FAILED. 0 passed; 3 failed
```

Cause: the shared `CARGO_TARGET_DIR` bakes `env!("CARGO_MANIFEST_DIR")` (from `crates/speaktype-session/tests/assemble_golden.rs:86`) into the integration-test binary. That binary was last built by another worktree; `strings …` shows
`/Users/ahsan/Desktop/speaktype/.claude/worktrees/agent-a66fbc54493f7cd5a/crates/speaktype-session`. Its golden files live there, so it cannot find ours.

Evidence it is pre-existing and environmental:
- `cargo test -p speaktype-session --test assemble_golden` **passes** (3/3) when `cargo test --workspace`/a later build refreshes the binary, and fails again as soon as the shared binary is reused.
- **Unmodified `main`** (`/Users/ahsan/Desktop/speaktype`, `8bbb514`) with the same `CARGO_TARGET_DIR` gets the **same failure with the same artifact hash** (`cargo make verify` → `EXIT: 2`, `assemble_golden-c9d708f4422b9b9c`).
- Adding `[profile.test] split-debuginfo = "off"` (probe only, reverted) makes the whole workspace pass: `80 / 3 / 27 / 3 …` all ok, no artifact collision.

Fix (not mine to make, root `Cargo.toml`): pin `split-debuginfo = "off"` for `profile.test`/`profile.dev`, or give each worktree its own target dir. Every other `make verify` target is green as listed above; I ran the UI targets individually because `make` stops at the failed Rust step.

## Review round 1

Merge `main` first (`4db7e7b`, the `CARGO_MANIFEST_DIR` fix) — clean, no conflicts.

**Must-fix — unbounded "Getting ready…":**

- `ServerGate::wait` is now `async fn wait(&self) -> bool`, bounded by `controller::SERVER_WAIT_MAX` (65 s = `server.rs` `STARTUP_TIMEOUT` 60 s + margin). `commands::ServerGate::wait` returns `false` early when the load task reports a failure (`AppState::model_load_failed`, set by `load_model_for_dictation`'s `Err` arm and cleared at each new load) or when the bound expires, so a capped-restart failure ends the wait immediately instead of after 65 s.
- On `false`, `Dictation::press` resets `pressing`, sets `DictationState { phase: Error, notice: serverError }` ("Speech model couldn't start") and does **not** start recording. `error` is an accepted starting phase, so the next press (hotkey or tray) is a fresh attempt rather than "already getting ready".
- `load_model_for_dictation` no longer emits the error state itself (the waiting press owns it; a late failure cannot blank a newer press's error).

**Tests added** (both in `controller::tests`, both paused clock):

- `dictation_reports_server_error_when_server_never_starts` — phases are `preparing → error` with the exact copy, nothing records, and a later press produces a fresh `preparing → error` (the latch was reset).
- `a_failed_load_ends_the_wait_early` — a gate that reports failure at 500 ms ends the wait there instead of at the 65 s bound.

**Commands (round 1, tails):**

```bash
git merge main                          # "Merge made by the 'ort' strategy."
cargo test -p speaktype --lib controller::   # "ok. 7 passed"
cargo fmt --all; cargo clippy -p speaktype --all-targets -- -D warnings   # clean
cargo test -p speaktype                 # "ok. 82 passed; 0 failed; 1 ignored"
cargo test -p speaktype export_bindings # "ok. 1 passed" (bindings unchanged)
make verify                             # EXIT 0 — 82 / 3 / 27 / 3 / 7 / 15 passed;
                                        # webui 56 + 95 tests; @speaktype/ui visual 33 + 28 passed
```

**Still needs a real build** (no app launch here): quit → first press shows "Getting ready…" → with the model file renamed to force a load failure, the pill shows "Speech model couldn't start · Try again" within ~65 s, and pressing again retries.

## Deviations from the plan and why

1. **History retention** — I added `settings.history_limit` early; the merge brought R1's authoritative field and I took main's whole `settings.rs`. `history::retention_limit` reads it (`None` = forever). No R2/R1 conflict remains.
2. **`dictation:long-audio`** instead of `long_audio` inside `dictation-state`: the take is still `transcribing` when >120 s audio is detected, and `done`/`error` notices would clear it. U2 can still render the fixed `longAudio` copy.
3. **Notice copy for three kinds** the plan table leaves blank: `pasted` → "Pasted"; `providerFallback` → "Switched to the speech model on this Mac"; `screenshotLimit` → "Screenshot limit reached". `screenshotLimit` is defined for U2/HUD to use. **`providerFallback` is not yet emitted anywhere**: `transcribe::emit_fallback` (`transcribe.rs:120-130`, outside R2's owned paths) still emits `sidecar:error {kind: 'providerFallback', …}`. U2 deletes that event; when it does, `emit_fallback` should call `commands::remember_dictation(app, DictationState::new(Done, Some(Notice::of(ProviderFallback))))`. Flagged in Open questions.
4. **Mic-levels event name** is `mic-levels` (`MicLevels` payload `{ levels: [{ device, level }] }`), matching U1's `MicPicker`. Each `DeviceLevel.device` is the cpal device name. Stopping also happens on window hide and app exit, because a closed picker in a hidden window cannot call `mic_levels_stop`.
5. **`delete_model`** also refuses ids containing `/`, `\` or `.` (path-traversal guard) as an idempotent `IpcError`; a missing file deletes nothing and succeeds.
6. **`paste_last`** records the re-paste outcome on the newest history entry (not just a toast).
7. **Hotkey registration is now owned by `EscRegistration`** (post-merge): Esc registers/unregisters through `global_shortcut`, dictation hotkeys still go through R1's `register_app_hotkey`, so R1's per-hotkey re-arm and screenshot keys are untouched.
8. **Tray and hotkey both route through `commands::press_dictation`**; the tray label follows the phase the press actually reached.

## New dependencies (and reasons)

| Dependency | Where | Reason |
|---|---|---|
| `tokio` `test-util` feature (**dev-dependency only**) | `apps/widget/src-tauri/Cargo.toml` | `tokio::time::pause/advance` for `mic_levels_auto_stop_after_30s` and `mic_levels_start_resets_auto_stop`. `full` does not include it; no runtime change. |

No production dependency was added.

## Manual checks I could not do (please check in a real build)

The brief forbids launching the app, so these are unverified:

1. **Esc during recording** cancels and the widget returns to idle; Esc does nothing when idle (registration is only armed on `recording`).
2. **Getting ready**: quit → first hotkey press shows the pill immediately, recording starts by itself ~2–16 s later (cold load), and the press is not lost.
3. **Mic picker**: open → every device's bars move; keep it open >30 s with the picker re-calling `mic_levels_start` every 20 s → bars keep moving; close → macOS microphone indicator turns off.
4. **Mic loss**: start recording on a Bluetooth mic, switch the headset off mid-take → recording continues and "Switched to MacBook microphone" shows.
5. **Paste failure**: revoke Accessibility → dictate → the transcript stays on the clipboard and "Copied. Press ⌘V to paste" shows; History row reads `pasteFailed`.

## Open questions

1. ~~Shared-target `assemble_golden` failure~~ — fixed on main (`4db7e7b`); `make verify` is green here.
2. **Legacy Settings "Restore defaults"** (`apps/widget/webui/src/settings/Settings.svelte:565`) still sets `post_paste_keys: 'enter'`, while R1's contract makes *Nothing* the new-install default. U4 replaces this page; flagging so it is not carried over.
3. **`dictation:long-audio` vs a `longAudio` notice in `dictation-state`**: confirm U2's preferred shape.
4. **`mic-levels` payload shape** (`{ levels: [{ device, level }] }`, device = raw cpal name): confirm this matches U1's `MicPicker` before U2 wires it.
5. **Event names**: the plan says `Event DictationState` and `MicLevels`; I emitted `dictation-state` and `mic-levels` to match the existing kebab-case typed-event convention (`session-state`, `settings-changed`). If the UI built against camelCase names, this needs a one-line rename in `ipc_export.rs`.
6. **`transcribe::emit_fallback` should move onto the typed state** when U2 deletes `sidecar:error` (see Deviation 3); that file is outside R2's owned paths, so I left it.
