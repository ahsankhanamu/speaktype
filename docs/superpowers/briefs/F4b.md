# Brief F4b: wire `speaktype-stt` into the app (Phase 0, Task 15)

**Read `_common.md` in this folder first.** Owner model: DeepSeek-V4.1-Flash. Worktree branch: `ws/F4b-stt-wire`.

**Goal:** the app transcribes through `speaktype_stt::provider_for`, using an explicit provider setting and Keychain-held keys, with the Cloud → On this Mac fallback.

**Your task:** Phase 0 plan **Task 15** exactly as written. Both crates are merged:
- `crates/speaktype-stt` is the provider library. Read its `lib.rs` for the exported API.
- Typed IPC uses tauri-specta.
  - Commands are `#[tauri::command] #[specta::specta]`, return `Result<T, IpcError>`, and are registered in the specta builder in `apps/widget/src-tauri/src/lib.rs`. Follow `session_cmds.rs` as the pattern.
  - Regenerate the bindings after adding commands: see `.handoff/F3/REPORT.md` for the exact command. Then commit `apps/widget/webui/src/lib/bindings.ts`.
  - Keep the registration test in `lib.rs` updated.

**Owned paths:**
- `apps/widget/src-tauri/src/transcribe.rs`
- The provider fields and migration in `apps/widget/src-tauri/src/settings.rs`
- `apps/widget/src-tauri/src/commands.rs`: the new `stt_*` commands only, or put them in a new `stt_cmds.rs`, which is preferred
- `apps/widget/src-tauri/src/lib.rs`: registration only
- `apps/widget/src-tauri/Cargo.toml` (add `speaktype-stt`), `Cargo.lock`
- `apps/widget/webui/src/lib/bindings.ts` (generated)
- `assets/test-clip-3s.wav` (a 3 s, 16 kHz mono clip of clear speech; generate it with macOS `say -o … --data-format=LEI16@16000` then convert, and say how)
- `.handoff/F4b/**`

**Notes from the reviewer:**
- Behaviour for local dictation must not change. Existing quality and hallucination-guard behaviour (`quality.rs`, `transcribe_verified`) keeps working on top of the provider's `Transcript.segments`.
- The fallback emits one `sidecar:error`-shaped event as the plan says. Don't add a new event channel.
