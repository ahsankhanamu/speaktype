# F3 — `speaktype-session` + typed IPC — Report

Workstream: F3 (owner model: zai-org/GLM-5.3) · Branch: `ws/F3-session-ipc` · Base: `c33953c`
Plan: `docs/superpowers/plans/2026-11-18-phase0-foundation.md`, Tasks 9–12. Spec: §6.1, §6.4, §6.5, §7.4, §10, §12.

## Tasks done

| Task | Commit | What |
|---|---|---|
| 9 — store with v2 schema | `16d2d85` | `crates/speaktype-session`: types (specta camelCase), `Store` over the kept `scribe_sessions`/`scribe_events` tables, v2 schema (`user_version = 2`, below-2 DBs dropped and recreated per spec §10), FTS5 over text/caption/`payload.ocr`, soft delete/restore/move/split (char-boundary), FTS search, 11 tests incl. the 8 named ones |
| 10 — prompt assembly | `9ea6aa2` | `assemble.rs` + golden files for all three targets (byte-for-byte, `{{bundle}}` placeholder for absolute paths), fence rule, >20-images + missing-image warnings, byte counts; 9 tests incl. the 5 named ones |
| 11 — bundle writer + image ops | `98a32c0` | `crates/speaktype-capture::downscale_png` (never upscales, 1568 cap, temp+rename, fast_image_resize Lanczos3) and `speaktype-session::bundle` (`slug`, `bundle_dir` with `.session-id` marker + `-2, -3…` suffixes, atomic `write_bundle` writing `images/NN-<kind>.png`, `prompt.md`, `session.json {schemaVersion:1,…}`, stale-image removal); 10 tests incl. the 6 named ones |
| 12 — typed IPC + Scribe removal | `dbad321` | tauri-specta builder over all 67 commands + 4 typed events, generated `bindings.ts` (freshness pinned by `export_bindings` test + `--ignored` writer variant), `session_*`/`item_*`/`prompt_*`/`bundle_reveal`/`overlay_*` commands with `IpcError {kind, message}`, `session_stop_emits_state_after_commit` (mock emitter sees the committed row via a second connection), clipboard/screenshot/dictation paths writing through `Store::add_item` on the active session (all spec §12 rules kept: caps, size limits, recent-dictation skip, 700 ms own-paste grace, 50-screenshot cap), `scribe.rs` + `ScribeTab.svelte` deleted, `llm.rs`/`llm_server.rs`/`templates.rs`/`docx_export.rs` kept compiled per spec §10 |

`make verify` exits 0 (see tails below). TDD was followed task-by-task: every named test was written first, watched fail (`todo!()`/compile/assert failures recorded in the session), then implemented; the golden tests were additionally corruption-tested (a modified golden byte fails the comparison).

## Exact commands run (tails)

```bash
npm ci            # "added 153 packages, and audited 156 packages in 4s"
cargo test -p speaktype-session        # Task 9 RED: "unresolved import … no `Store` in `store`" → GREEN: "test result: ok. 10 passed"
cargo test -p speaktype-session        # Task 10 RED: 6× "not yet implemented" → GREEN: "ok. 16 passed" + "ok. 3 passed" (golden)
cargo test -p speaktype-capture -p speaktype-session bundle   # Task 11 RED: 10× "not yet implemented" → GREEN
cargo test -p speaktype every_frontend_command_is_registered # Task 12 RED: "frontend calls commands not registered … session_start …" 18 names
cargo test -p speaktype export_bindings -- --ignored --nocapture   # regenerates bindings.ts; "test result: ok"
cargo test -p speaktype export_bindings  # "test result: ok. 1 passed" + `git diff --exit-code apps/widget/webui/src/lib/bindings.ts` → clean
cargo test -p speaktype                 # "ok. 36 passed; 0 failed; 1 ignored" (ignored = the writer variant)
cargo test --workspace                  # ok — 36 + 3 + 25 + 3 across crates
npm -w speaktype-webui run check        # "svelte-check found 0 errors and 0 warnings"
npm -w speaktype-webui test             # "Test Files 4 passed (4) · Tests 31 passed (31)"
make verify                             # "EXIT: 0"; ends "33 passed" (@speaktype/ui visual)
```

Environment prerequisites for a local `make verify` (pre-existing on main, verified via `git stash` baseline): tauri-build needs `apps/widget/src-tauri/binaries/whisper-server-aarch64-apple-darwin` (git-ignored sidecar) and `apps/widget/webui/dist`. I created a git-ignored placeholder sidecar stub and built the webui once; neither is committed, and real builds overwrite the stub. CI builds the real sidecar before verify.

## New dependencies (and removals)

| Dependency | Where | Reason |
|---|---|---|
| `specta =2.0.0-rc.28` (exact; `derive`, `serde_json`) | speaktype-session, app | Required *exactly* by tauri-specta rc.29; one version workspace-wide so the `specta::Type` derives and the TS export agree |
| `tauri-specta =2.0.0-rc.29` (`derive`, `typescript`) | app | Typed IPC generation for Tauri 2 — the Task 12 deliverable; newest Tauri-2-compatible release (2026-11-13) |
| `specta-typescript =0.0.15` | app | tauri-specta's TypeScript export backend (its required version) |
| `unicode-normalization 0.1` | speaktype-session | NFKD folding for session-name slugs (plan: "NFKD → ASCII") |
| `fast_image_resize 6` (`image` feature) | speaktype-capture | The plan-named resize library for the ≤ 1568 px downscale (Lanczos3) |
| `image 0.25` png-only | speaktype-capture | PNG decode/encode (plan: "image with png only"); dev-only in speaktype-session for test fixtures |
| `thiserror 2` | app (direct) | `IpcError` derive (was already in the lock transitively) |
| **removed** `screencapturekit` | app | Its only use (the Scribe screen-recording preflight) was deleted with the scribe commands; Phase 2's SCK freeze-frame re-adds it |

Lockfile (not manifest): `tauri 2.10.2 → 2.12.7` — tauri-specta rc.29 references commands via tauri's `__tauri_command_name_<fn>!` hidden macro, which only exists from tauri-macros 2.7 (tauri 2.11+); on 2.10.2 every `collect_commands!` entry failed to resolve. `gen/schemas/*` regenerate with it. Full suite and `make verify` are green on 2.12.7.

## Deviations from the plan (rulings)

1. **`Store::item(id)` added** beyond the plan's fixed interface — `item_restore` must emit `SessionItemAdded` with the restored row (fixture-tested).
2. **Event names are kebab-case** (`session-state`, `session-item-added`, …) — tauri-specta derives them from the struct names; the spec §12 "scribe:* → session:*" rename is realised as these typed events. `bindings.ts` is the contract; Phase-1 surfaces consume `events.*`.
3. **`IpcError` only on the new commands.** The old surface keeps `Result<T, String>` (ErrorHandlingMode::Throw preserves today's reject-with-string behaviour). Normalising the old surface to `{kind, message}` is spec §6.1 = Phase-1 D1 widget work; doing it here would churn every unowned caller.
4. **`prompt_render` materialises the bundle** (write, then re-assemble so byte counts measure real files). Phase 0 has no session engine, so rendering is the materialisation point of spec §6.5 ("on Stop and on edits"); Phase 2's engine takes this over.
5. **`bundle_dir` keeps the plan's `PathBuf` signature**, so dir/marker allocation is best-effort; real fs errors surface in `write_bundle`, which propagates. Suffixes are `-2, -3…` per plan (my first draft started at `-1`; the failing test caught it).
6. **Golden files use a `{{bundle}}` placeholder** for the coding-agent target (absolute paths are per-run); the test substitutes the temp dir before the byte-for-byte compare. Golden comparison was corruption-tested.
7. **Unowned files edited** (flagged in Open questions): `Widget.svelte` (scribe strip was dead UI over removed commands; spec §12 says the HUD replaces it — Phase 2 S3), `DebugTab.svelte` (one-line type fix: `chars?: string` → `number`; Rust sends usize), `Settings.test.ts` (7 tabs → 6). All type-surface only, no logic changes. `settings.css` scribe rules were left for the Settings rebuild (F2/Phase-1 ownership of that file's shrink).
8. **Kept-compiled modules** (spec §10): `scribe.rs` is deleted; `llm.rs`/`llm_server.rs`/`templates.rs` are `#[allow(dead_code)]` with a spec §10 citation, minimally adapted to the new store; `templates.rs` keeps the template data + `template_prompt`/`infer_template` (used live by `prompt_render` — "auto" infers from content) and drops its ScribeStore-coupled renderers (superseded by `assemble`). `docx_export.rs` is kept the same way for Phase 3 L3 ("Export .md / .docx", spec §6.3) and now writes to `~/Documents/SpeakType/Exports` (spec §12). Reviewer may overturn and delete it instead.
9. **`session_start` template default** = `settings.scribe_template` (falling back to `"auto"`), name = `"Session <Mon D HH:MM>"` (renamable; first-words naming is the Phase-2 narration engine's).
10. **Captions do not appear in the assembled prompt** — §6.4's format rules mention none; the Timeline UI owns captions.
11. **`session_rename`/`session_delete` emit no event** — the plan defines exactly four events; window-refresh for renames arrives with F5's liveQuery invalidation.
12. **`disable_serde_phases` + `dangerously_cast_bigints_to_number`** on the builder: the app's shapes are symmetric (plain objects match the old interfaces), and the only wide integers are SQLite ids, epoch-ms and byte counts — every real value ≪ 2^53. The plan fixes them as `i64`/`u64`.
13. **`ipc.ts` is a compat shim**: `export * from './bindings'` + typed alias re-exports (pure casts — zero hand-written `invoke`) + `listenEvent`/`emitEvent` (F5 Task 16 owns migrating the one `listen()` site). Value-returning commands keep their hand-written payload interfaces as cast targets. Moving every caller to `commands.*` directly would touch ~8 unowned Svelte surfaces; Phase 1 rebuilds exactly those.
14. **specta models `#[serde(default)]` f64s as `number | null`** (Rust never sends null for these non-Options). The `getDebugSessions` alias narrows them via a documented mapped type; genuine `Option` fields keep their null. If a future specta fixes the modelling, the shim shrinks.
15. **`settings.scribe_*` fields are retained** (incl. now-unread `scribe_enabled`) — the settings cleanup belongs to F4/Phase 1; removing fields mid-parallel-work would churn F4's file. Clipboard gating is now `scribe_clipboard_monitor` + active-session presence (mode switch gone).
16. **`Item.image_path` is absolute** (bundle copies straight from it); raw captures stay in `<config>/scribe/<id>/` (spec §12), bundles in `~/Documents/SpeakType/Sessions` (§6.5).

## Open questions

1. **Unowned-file edits** (`Widget.svelte`, `DebugTab.svelte`, `Settings.test.ts`): Task 12's "remove every `*_scribe_*` command" + "npm run check passes with every caller moved to bindings.ts" cannot be met without them (the strip called removed commands; the local type hid a misannotation). Please confirm the scope is acceptable or tell me to re-shape it.
2. **`session_rename`/`session_delete` produce no invalidating event** — F5's `liveQuery` for the session list will need one (or a refetch trigger). Should a `SessionRenamed`/`SessionDeleted` event be added now or with F5?
3. **The ipc.ts alias shim vs the "no hand-written IPC" review gate**: aliases are pure re-exports of the generated functions; full caller migration to `commands.*` is Phase-1 surface work (D1/D3). Flagging so the gate is applied with that plan in mind.
4. **`docx_export.rs` kept under `#[allow(dead_code)]`** for Phase 3 L3 — or delete it and re-derive from git when L3 lands?
5. **Local verify prerequisites** (placeholder sidecar + webui dist) are undocumented developer-setup gaps on main — maybe `make verify` should build the webui and tolerate a missing sidecar (or the docs should state the prerequisite). Not fixed here: outside owned paths.
6. **`session_cmds::capture_screenshot` drops the old scribe-mode error strings** ("Scribe mode is off…") in favour of silent no-ops when no session is active — per Task 12 ("or no-op when there isn't one"), but the 50-cap message now only logs (HUD message is Phase 2).

Screens: none — F3 is backend/IPC; no UI surfaces were added. Rulings ledger: `.superpowers/sdd/2026-11-18-phase0-foundation/progress.md` (working-tree scratch, not committed).

## Review round 1 (fixes, commit `fix(session): review round 1`)

All five findings fixed, each with a test; `make verify` exit 0 after (cargo 70 tests, webui check 0 errors, webui 31 tests, ui visual 33 passed; bindings fresh).

1. **`session_start` is idempotent** — new `start_session` seam: an already-active session is returned as-is and nothing is created or emitted (the rapid-hotkey case). Test `session_start_is_idempotent`: two starts → same id, exactly one row with `ended_at` NULL, emitter ran once.
2. **`prompt_render` is read-only** — `render_prompt` no longer writes anything and assembles against a new read-only `bundle::bundle_dir_readonly` (existing marker wins, otherwise the candidate path is returned uncreated). Byte counts reflect reality: unwritten images count 0 and are named in `warning`. The bundle is materialised by the new `bundle_write` helper, called by `session_stop` after `end_session` commits (the Phase 3 timeline editor will call the same helper). Tests: `prompt_render_is_read_only` (no directory created, 0 bytes, "Image 1 is missing.") and `stop_writes_the_bundle_after_commit` (stop → `prompt.md` + `images/01-mark.png` exist; `session_stop_emits_state_after_commit` keeps proving commit-before-notify). `bundle_reveal` now uses the read-only path and errors with kind `bundle` when the folder isn't written yet. Side fix: `render_prompt`/`bundle_write` take the sessions root as a parameter, so tests run against temp roots — the previous test briefly wrote into `~/Documents/SpeakType/Sessions`.
3. **`SessionsChanged {}` event** (session-list invalidation) registered in the builder and emitted after the DB commit by `session_start` (only when a session is actually created), `session_stop`, `session_rename` and `session_delete`. Bindings regenerated: `events.sessionsChanged` → `"sessions-changed"`. Test: `session_events_are_registered_in_bindings` asserts all five session events in `bindings.ts`. (The typed emit calls themselves are statement-ordered after the store calls; they can't be unit-tested because `tauri::Emitter` is sealed and a Wry mock app panics in tao off the main thread on macOS — the commit-ordering guarantee is still proven through the `stop_and_notify` seam.)
4. **`docx_export.rs` and the `docx-rs` dependency deleted** (unused code under the lean rules; git keeps it for Phase 3). `png_dimensions`'s test moved to `capture.rs`. `llm.rs`/`llm_server.rs`/`templates.rs` stay per spec §10. This resolves open question 4.
5. **Screenshot cap surfaces as an error** — `capture_screenshot` returns `Err(IpcError { kind: "screenshotLimit", message: "This session has 50 screenshots, the most it can hold." })` (max interpolated) instead of only logging; a missing active session stays a silent no-op. The hotkey handler logs the error until Phase 2's HUD consumes it. Test: `screenshot_limit_error_shape` pins the kind and copy.

Open question 2 (list invalidation) is answered by fix 3. The remaining open questions are 1 (approved — closed), 3 (alias shim plan, accepted), 5 (local verify prerequisites on main) and 6 (superseded by fix 5's error surface).
