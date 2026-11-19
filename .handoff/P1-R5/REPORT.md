# P1-R5 — shortcut registry (Phase 1, Task 8)

Branch `ws/P1-R5`, cut from `eb64928`. Owner model DeepSeek-V4.1-Flash. No subagents; nothing was
launched (no `make dev`, no `cargo tauri dev`, no built `.app`).

## Tasks done

| # | What | Commit |
|---|---|---|
| 1 | **Brief, Task 8 and spec §5 carried onto the branch.** They only existed on `u3-base`; the brief arrived by cherry-pick, the plan/spec additions by patch (only `docs/` differed). | `9f11b1b`, `b4f08b3`, `86571d3` |
| 2 | **`shortcuts.rs` core:** `ShortcutAction` (serde camelCase), `ShortcutBinding`, defaults, stored spelling (modifiers deduped in Apple order), conflict check with UI action names, the macOS-shortcut table, force, restart-safe defaults, the `ShortcutsPlatform` trait and the `apply` applier. | `cb284f6` |
| 3 | **Settings:** one `shortcuts: BTreeMap<ShortcutAction, String>`; migration from `hotkey`, `scribe_screenshot_hotkey`, `session_hotkey`, `draw_key` keeping every value; the four fields removed (old files still parse). | `98742bf` |
| 4 | **Registration path + commands:** `AppShortcuts` (tap for the dictation chord, plugin otherwise) in `hotkey.rs`; `shortcuts_get`, `shortcut_set(action, keys, force)`, `shortcuts_reset`; `ShortcutsChanged`; `update_hotkey` and its `hotkey:registered` event gone; screenshot registration moved out of `clipboard.rs`; `unregister_all` only on exit; `onboarding:finished.hotkey` from the registry. | `98742bf` |
| 5 | **Frontend:** `bindings.ts` regenerated; `updateHotkey` removed; Dictation, Sessions, History, the widget and onboarding's Try it read `settings.shortcuts`; writes go through `shortcut_set`; visual fixtures updated. | `aa3d9f3` |
| 6 | **Verification:** mock fix that `make verify` needed; two review fixes (unreadable legacy value, `SettingsChanged`). | `3541733`, `c37afc2` |

Contract tests, all green: `migration_keeps_existing_values` (F8 fixture), `set_conflicting_shortcut_is_refused`,
`set_reregisters_only_that_action` (fake registry: only that action's key changes, the tap is not restarted,
`unregister_all` never runs during a change), `register_failure_keeps_previous`, `reset_restores_defaults`,
`system_shortcut_needs_force`, `u5_name_and_stored_values_both_validate_to_the_stored_draw_key`.

## Commands run (tail of each)

```
npm ci                                     → ok (esbuild/fsevents install-script warnings only)
npm --prefix apps/widget/webui run build    → ✓ built in 6.13s
cargo test -p speaktype --lib shortcuts::  → test result: ok. 10 passed … (RED before each step: functions missing; then 8 failed/2 wrong
                                             expectations traced to real bugs — key-name leak, out-of-order dedup)
cargo test --workspace                     → test result: ok. 112 passed; 0 failed; 1 ignored
cargo clippy --workspace --all-targets -- -D warnings → no output (clean)
npx svelte-check                           → svelte-check found 0 errors and 0 warnings
npm -w speaktype-webui run lint            → clean
npx vitest run                             → Test Files 13 passed (13) · Tests 161 passed (161)
npm -w speaktype-webui run visual          → 96 passed (first run: one widget-recording waveform diff; baseline alone
                                             passes and a rerun passed — animation phase, not this change)
make verify                                → exit=0
   cargo fmt --check, clippy -D warnings, cargo test (112), webui check/lint/test (161),
   @speaktype/ui visual (33), webui visual (96)
```

## Deviations from the plan, and why

1. **Docs carried onto the branch.** `docs/superpowers/briefs/P1-R5.md`, Task 8 and spec §5 were on
   `u3-base` only, so this worktree could not read its own task. The brief came by cherry-pick (identical
   commits); the plan/spec edits were on `u3-base` only at that point, so I applied that exact diff. No
   other doc content changed.
2. **Fixed the two `Task 8` wiring facts the plan left open.**
   - The **session key is now registered** (it was only stored). It starts a session, or stops the running
     one. The plan's "one registration path: … session, screenshot via the global-shortcut plugin" needs it.
   - The **screenshot key** moved out of `clipboard.rs` into the registry. `clipboard.rs` is listed as owned
     only for "screenshot registration"; that function is now the registry's, so it was deleted there.
3. **Frontend files beyond `updateHotkey` callers.** The contract *removes* the four old fields, so
   `Dictation.svelte`, `Sessions.svelte`, `History.svelte`, `search`-adjacent fixtures, `Widget.svelte` and
   onboarding's Try it would not type-check otherwise. They now read `settings.shortcuts`; the two writes go
   through `shortcut_set`.
4. **`force` is accepted and stored, never refused.** There is no tap yet to hold a system draw key, so the OS
   rejection (`registerFailed`) is the only refusal. Marked in the code.
5. **`save_settings` no longer re-registers anything.** Shortcuts are the registry's; `save_settings` writes
   and emits.
6. **Two existing surface-list gaps fixed:** `paste_last`, `delete_model`, `mic_levels_start/stop` were
   registered in `ipc_export.rs` but missing from the `ipc_surface` lists. They are listed now.
7. **`shortcuts` has `#[serde(default)]` but no `skip_serializing_if`:** a file without the map still parses,
   a migrated one always writes it, and the UI's generated type reads it as optional.

## New dependencies

None.

## What the reviewer should check in the real app

- **A fresh launch with the owner's file:** the dictation key is still **F8** after the first write (check
  Settings → Dictation and onboarding's Try it), and settings.json now has a `shortcuts` object
  (`{"dictation":"F8",...}`) with no `hotkey`/`draw_key`/`scribe_screenshot_hotkey`/`session_hotkey` keys.
- **Changing one key leaves the others working:** move Start / stop a session, then confirm dictation (tap
  chord or F8) and ⌥⌘S both still fire.
- **A key another app owns** (e.g. bind ⌥⌘K to another app first): the UI shows "Couldn't use this shortcut.
  Another app may own it." and the previous key still works.
- **⌘Space:** first attempt warns "macOS uses this for Spotlight"; a forced resend is accepted and works.
- **The chord hand-over:** F8 → ⌃⌘ stops the plugin key and starts the tap in one change; ⌃⌘ → F8 does the
  reverse; neither disturbs the session/screenshot keys.
- **Esc during recording** still cancels (its own path, unchanged).
- **The draw key:** Settings → Sessions writes `right_option`/`right_command`/`fn` into `shortcuts.drawKey`;
  nothing registers it yet (Phase 2).

## Open questions

1. **Real app window.** "Take a screenshot" fires `capture_screenshot`, which ignores a missing session
   (logs "Screenshot hotkey with no active session — ignored"). Spec §5 says it "joins or starts a session";
   auto-starting a mic session from a screenshot hotkey is a product decision (the session starts listening),
   so I left the behaviour as it is. Needs a call.
2. **`ShortcutsChanged` vs `SettingsChanged`.** `shortcut_set`/`shortcuts_reset` emit both (U5 listens for
   `shortcuts-changed`; the pages I moved listen for `settings-changed`). If U5 would rather have one, drop
   the settings emit.
3. **U5 merge overlap.** U5 adds `app-window/shortcuts.ts` and moves the Dictation row to the Keyboard page.
   This workstream changed the same pages; expect a conflict in `Dictation.svelte` and `Sessions.svelte`.
4. **macOS system-shortcut list** is the spec's (⌘Space, ⌘Tab, ⌘Q/W/H/M, ⌃Space, ⌘⇧3/4/5). Should ⌘⇧4's
   block reveal be handled, or ⌘⌥Esc? Not in the spec's list.
