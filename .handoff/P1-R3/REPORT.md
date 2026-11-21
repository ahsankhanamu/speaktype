# P1-R3 report — Task 7: window and command gaps

Branch `ws/P1-R3`, owner model DeepSeek-V4.1-Flash. Worktree `/Users/ahsan/Desktop/speaktype-ws/P1-R3`
(CARGO_TARGET_DIR=/Users/ahsan/Desktop/speaktype-ws/.target for every cargo/make call).
Fresh-worktree setup done first: `npm ci`, `npm --prefix apps/widget/webui run build`,
binaries copied from `/Users/ahsan/Desktop/speaktype/apps/widget/src-tauri/binaries/`.

## Tasks done, with commits

| Commit | What |
|---|---|
| `814ccd2` | **Settings fields** `session_hotkey = "Option+Super+K"`, `live_captions = true`, `keep_session_audio = false`, `widget_corner = "bottom_right"`; **`set_launch_at_login` persists `launch_at_login`** (Rust is the single source of truth) and emits `SettingsChanged`. |
| `fd1f3b8` | **Widget corners** (`corner_origin(visible, size, corner)` for all four corners, 16 pt inset, `WidgetCorner` including `custom`, `set_widget_corner` on the active display's *visible frame* (`work_area`, so never under the menu bar or Dock), startup honours the stored corner, a drag stores `custom`, the tray "Show Widget" is the only reveal and it places the widget at its corner). **About route**: `open_about` and the tray About item open the main window at `#/about`; the `about` window is no longer created. **`test_typing`**, **`screen_recording_status`**, **`request_screen_recording`**. **`HistoryChanged {}`** after add / delete / delete-audio / clear / re-paste / reprocess. **`reprocess_preview(index)`**. **`MicDevicesChanged {}`** (typed successor of `sidecar:device_changed`). Window chrome: main window and onboarding use `titleBarStyle: Overlay` + hidden title; onboarding is 960 × 640, centred, not resizable. Regenerated `bindings.ts` (additive only). |
| `4127bcd` | Onboarding's Try it compare now calls `reprocessPreview(0)` instead of `reprocessHistoryEntry(0)` (the old call needed saved audio and overwrote the entry). |
| `e3535a4` | Visual fixture for the Try it compare answers `reprocess_preview` with the new return shape. |
| `cd95d33` | Harness server pinned to `127.0.0.1:5198` in `playwright.config.ts` + `tests/visual/vite.harness.config.ts` (see Deviations 4). |

### Failing tests first (TDD), then implementation

Each was written and run red before the code, then green:

- `settings_defaults_for_new_install` (extended: the four new fields), `settings_reads_the_session_and_widget_fields_from_an_existing_file`,
  `settings_roundtrip_session_and_widget_fields`, `set_launch_at_login_fails_as_ipc_error_without_an_app_handle`
  (extended: a failed enable does not record `launch_at_login = true`).
- `window::tests::widget_corner_frames` (`corner_origin` for all four corners), `corner_origin_uses_the_offset_display_and_the_inset`, `widget_corner_wire_names`.
- `history::tests::delete_entry_emits_history_changed_and_persists` (real file in the test config dir: delete persists *and* the sink gets one `HistoryChanged`).
- `commands::tests::about_url_is_the_windows_route`, `about_url_replaces_another_route`.
- `widget_cmds::tests::unknown_corners_are_rejected`, `widget_corner_changed_keeps_its_wire_shape`.
- `paste::tests::test_typing_text_is_one_plain_line`.
- Registration test updated: `REGISTERED` / `FRONTEND_USED` gained `set_widget_corner`, `screen_recording_status`,
  `request_screen_recording`, `test_typing`, `reprocess_preview`; `every_frontend_command_is_registered` also passes.
- `onboarding.test.ts` "re-run with another model" rewritten to expect `reprocessPreview(0)` (3 red → green).

**Task 6's History page note:** deletes come through `delete_history_entry` / `clear_history`, which now emit
`HistoryChanged`. `item_delete` / `session_delete` do not — that is the session domain's own event; say if History wants it.

## Commands run (tails)

```bash
export CARGO_TARGET_DIR=/Users/ahsan/Desktop/speaktype-ws/.target
npm ci && npm --prefix apps/widget/webui run build     # SETUP_OK, "✓ built in 7.06s"
cargo test -p speaktype --lib settings::               # 14 passed
cargo test -p speaktype --lib window::                 # 3 passed (red first: E0308 work_area signature)
cargo build -p speaktype                               # Finished dev profile
cargo test -p speaktype --lib                          # 93 passed; 1 failed (stale bindings) → after regenerate: 93 passed, 1 ignored
cargo test -p speaktype --lib export_bindings_write -- --ignored   # 1 passed
cargo fmt --all --check                                # clean after `cargo fmt --all`
cargo clippy -p speaktype --all-targets -- -D warnings # clean
(cd apps/widget/webui && npx vitest run src/onboarding/onboarding.test.ts)   # 22 passed
(cd apps/widget/webui && npm run check && npm run lint && npm test)          # 0 errors; 95 tests passed
(cd apps/widget/webui && npx playwright test)          # 28 passed
make verify                                            # exit=0
```

`make verify` tail:

```
test result: ok. 93 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out
test result: ok. 27 passed; 0 failed; 0 ignored        (speaktype-session)
test result: ok. 15 passed; 0 failed; 0 ignored        (speaktype-stt)
svelte-check found 0 errors and 0 warnings
 Test Files  5 passed (5)   Tests  56 passed (56)      (@speaktype/ui)
 Test Files  9 passed (9)   Tests  95 passed (95)      (webui)
  33 passed (20.1s)                                     (@speaktype/ui visual)
  28 passed (24.7s)                                     (webui visual)
```

## Deviations from the plan

1. **`reprocess_preview` does not load the model server.** It only pre-warms nothing and uses `settings.api_url`, which
   `server.rs` keeps pointing at the live port. A compare that restarted the server would make "Back to <previous>" a
   restart rather than a no-op. Onboarding's Try it only compares after a dictation, so the server is already up.
2. **`reprocess_history_entry` now shares `History::reprocess`** (one implementation, the entry is overwritten by the
   caller). Its result text and errors are unchanged; it gained the `HistoryChanged` emit.
3. **`sidecar:device_changed` → typed `MicDevicesChanged` (`mic-devices-changed`).** It is not on the R2 delete list and the
   device watcher lived in `lib.rs` (outside my paths). I moved the watcher into `widget_cmds.rs` and made the event typed;
   nothing in the webui listens to the old name today. The reviewer should confirm U2/U4 adopt `events.micDevicesChanged`.
4. **Visual harness server pinned to `127.0.0.1`.** On this Mac `localhost` resolves to `::1`; `make verify` failed with
   `ERR_CONNECTION_REFUSED` on 19 of 28 webui visual tests because vite bound another address (and port 5198 is shared with
   other worktrees). Two config lines; both point at the same address now.
5. **`save_dragged_position` is a native `WindowEvent::Moved` handler**, not a webview command. The plan's files did not
   include `lib.rs`, but the window-event handler lives there; the change there is limited to that handler, the device
   watcher call, the tray "show"/"about" closures, the startup corner placement and `last_dictation` in `AppState`.
6. **First-show corner:** the plan did not say which corner "Show Widget" restores. I chose bottom-right (the same place as
   Reset widget position). The setting is the visible position glyphs' choice, and `Open settings`, the dock reopen and
   startup never move the widget.
7. **`onboarding_window_x/y` are now unused in settings** (onboarding is centred every launch, so the save path was
   deleted). Fields kept for file compatibility.
8. **Removing UI stand-ins:** `apps/widget/webui/src/app-window/pending.ts` is not on `main` (U4 unmerged), so per the brief I
   list stand-ins instead: `src/settings/*` (old Settings window) and `src/about/*` + `about.html` still on `main`. The
   onboarding cast is already gone. The Try it compare's `reprocessHistoryEntry` stand-in **is** replaced.
9. **`about.html` stays built** by `vite.config.ts` until U4's `src/app-window` replaces it; `open_about` no longer opens
   that window.

## New dependencies

None. `CGPreflightScreenCaptureAccess` / `CGRequestScreenCaptureAccess` are linked from the already-used CoreGraphics framework.

## Manual checks for the reviewer (never launching the app here)

- **Test typing:** Permissions page → Test typing types "SpeakType test" into its own field with Accessibility on; with
  Accessibility off the command returns `IpcError{kind:'accessibility'}`.
- **Screen Recording:** row shows the real `CGPreflightScreenCaptureAccess` status; "Open System Settings…" via
  `request_screen_recording` adds SpeakType to the list (macOS only shows the grant after a relaunch).
- **Widget corners:** Appearance → Widget position moves the widget to each corner of the active display's visible frame
  (16 pt inset); drag it, reopen Settings → "Where I last dragged it" is checked; Reset widget position → bottom-right; quit
  and relaunch → the chosen corner is kept.
- **Window chrome:** the main window and onboarding show the traffic lights over the content with no title bar and no title
  text; onboarding opens centred at 960 × 640 and cannot be resized.
- **About:** Settings sidebar app row and the tray's About SpeakType both bring the main window forward at `#/about`
  (needs U4's router).

## Open questions

- Does U4's router accept `#/about` from `window.navigate`, or does it need `location.hash` set a different way?
- Should History also refetch on `session-items-deleted` / `sessions-changed` (task 6 text), i.e. delete a session entry from
  History → list updates? That needs an emit from the session commands, outside my paths.
- Is `MicDevicesChanged` the right name for the device event (U1's `MicPicker` / U4's Dictation page will listen to it)?
