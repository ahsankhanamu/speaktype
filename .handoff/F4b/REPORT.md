# F4b — wire `speaktype-stt` into the app — Report

Workstream: F4b (owner model DeepSeek-V4.1-Flash) · Branch `ws/F4b-stt-wire` · Base `33fb6a2`
Plan: `docs/superpowers/plans/2026-11-18-phase0-foundation.md` **Task 15**. Brief: `docs/superpowers/briefs/F4b.md`.

## Tasks done

| Commit | What |
|---|---|
| `8d47d3d` `feat(stt): provider_for takes the running server address` | **Contract change ruled by the owner** (see Deviations 1): `provider_for(cfg, local_url, secrets, http)`; `LOCAL_ENDPOINT` deleted; `Local{model}` uses `local_url`; crate test `local_uses_given_url` added (RED: `E0061`/`E0277`, GREEN: 14/14). App side: `transcribe.rs` rewritten as a thin adapter over `provider_for` (old HTTP path and `is_openai_api` deleted), Cloud → On this Mac fallback (`Unreachable`/`Timeout` only, and only with `fallback_to_local`), quality/hallucination ladder kept on `Transcript.segments`; `settings.rs` gains `stt_provider` + `from_json` migration (default `api_url` → `Local`, other → `Server`) and moves a plaintext `scribe_llm_api_key` into the Keychain (`llm.api_key`); `AppHandle` threaded through `chunk_session`/`commands`; `assets/test-clip-3s.wav` (3.295 s, 16 kHz mono s16). |
| `49b84fb` `feat(stt): providers page IPC and the Test clip` | `stt_cmds.rs`: `stt_set_provider`, `stt_set_api_key` (Keychain only), `stt_has_api_key`, `stt_reveal_api_key` (Keychain read → macOS prompt), `stt_list_models`, `stt_test` (bundled clip → `{ latencyMs, text }`), all `#[specta::specta]` + `IpcError`, registered in `ipc_export::builder()`; `bindings.ts` regenerated; bindings-registration test. |
| (this commit) `docs(handoff): F4b report` | This report (`.handoff/F4b/REPORT.md`). |

Named tests, all written first and seen fail (RED noted inline):
- `settings_migrates_default_url_to_local` — `settings.rs` (RED: `Settings::from_json` missing → `E0599`).
- `settings_migrates_custom_url_to_server` — same.
- `llm_key_moves_to_keychain_and_blanks_json` — `MemoryStore` injected through the `SecretStore` parameter (RED: same).
- `cloud_unreachable_falls_back_to_local_when_enabled` — two independent servers: the cloud side is a closed port (`127.0.0.1:9`), the local side is a wiremock `MockServer` serving the real whisper-server verbose_json fixture (RED: `unimplemented!`/missing args).
- Extras: `cloud_unreachable_without_fallback_is_one_plain_message`, `only_unreachable_cloud_failures_fall_back`, `service_names_are_plain_language`, `accounts_cover_each_service_and_the_llm_key`, `test_clip_is_three_seconds_of_16khz_mono`, `stt_commands_are_registered_in_bindings` (RED: bindings lacked all six names), crate `local_uses_given_url`.

## Exact commands run (tails)

```bash
npm ci                                              # "added 153 packages, and audited 156 packages in 4s"
npm --prefix apps/widget/webui run build            # "✓ built in 5.37s"
cp /Users/ahsan/Desktop/speaktype/apps/widget/src-tauri/binaries/* apps/widget/src-tauri/binaries/
cargo test -p speaktype                             # baseline before edits: "38 passed; 0 failed; 1 ignored"

# clip (see Deviations 5 for the exact recipe)
say -v Samantha -o /tmp/st-3c.wav --data-format=LEI16@16000 "This is a test of the SpeakType speech service, testing."
python3 …trim leading/trailing silence, write assets/test-clip-3s.wav   # "duration 3.2954375"
afinfo assets/test-clip-3s.wav                      # "1 ch, 16000 Hz, Int16 … 3.295437 sec"

cargo test -p speaktype-stt --test providers        # RED: "E0277 String: SecretStore", "E0061 takes 3 arguments but 4 supplied"
cargo test -p speaktype-stt                         # GREEN: "ok. 7 passed" + "ok. 14 passed" incl. local_uses_given_url
cargo test --lib transcribe::                       # RED→GREEN: "ok. 4 passed"
cargo test --lib ipc_export::tests::stt_commands    # RED: bindings.ts must export sttSetProvider
cargo test --lib export_bindings_write -- --ignored # regenerate bindings.ts
cargo test --lib                                    # "44 tests … 43 passed; 1 ignored"
cargo clippy --workspace --all-targets -- -D warnings   # clean
for f in settings_migrates llm_key_moves cloud_unreachable …; do cargo test -p speaktype --lib -- "$f"; done
make verify                                         # "EXIT=0"
```

`make verify` was also run once under machine load 37–43 (other workstreams compiling): the webui
Settings mount test timed out (3 failures) while every crate stayed green; the same file passed 3/3 in
isolation and the full gate passed at load ≈5. Same source, no change between runs.

`make verify` tail: `cargo fmt --all --check` clean · clippy clean · `test result: ok. 43 passed … 1 ignored`
(app) · `ok. 3` (capture) · `ok. 27` (session) · `ok. 3` (session golden) · `ok. 7` + `ok. 14` (stt) ·
`svelte-check found 0 errors and 0 warnings` (both workspaces) · webui `Tests 56 passed (56)` ·
ui visual `33 passed (15.6s)`.

Clip sanity (no whisper server was running locally, so no end-to-end decode was attempted):
`3.295s 16000 Hz peak=21846 rms=4252 silent_windows=2/33`, with syllable-rate RMS variation
(`6538 3108 5929 4986 1992 5927 …`) and two inter-word dips (`307 21 2 2430`) — speech, not a tone.

## Deviations from the plan (and why)

1. **`provider_for` signature changed (owner ruling).** The plan's `provider_for(cfg, secrets, http)`
   hard-coded the local endpoint to port 8002, but `server.rs` picks the port at start and keeps it in
   `settings.api_url` (e.g. 8003 in the local log). The owner ruled: pass `local_url` in. Done in the
   crate; `LOCAL_ENDPOINT` deleted; crate callers and tests updated; app passes `settings.api_url`, and the
   fallback reuses the same `local_url`. This touches `crates/speaktype-stt/**` (outside the brief's owned
   list) under that explicit ruling.
2. **The hallucination ladder is one retry, not three passes.** The plan-pinned `TranscribeRequest` has
   no decode-parameter fields (F4 report open question 1), so the old temperature ladder (0 → drop prompt →
   0.4 + internal fallback) cannot be expressed. What remains: pass 1 sends the carried-over prompt; if the
   result is not clean and not silent, exactly one re-decode without the prompt; best score wins. The
   scoring, dropping, `quality::Thresholds` and debug `PassEvent` recording are unchanged. Consequence:
   a degenerate greedy repetition loop with no prompt is no longer broken by a temperature bump.
   Needs a ruling (Open question 1).
3. **Local provider copy name vs. the spec.** Fallback copy is exactly as the brief/plan says
   (`"Couldn't reach <service>. Used On this Mac."`); `service_name(Local)` is `"On this Mac"` too, so a
   local failure reads `"Couldn't reach On this Mac."`. The spec (`6.7`, `transcribe_verified` line)
   says "use On this Mac" for the switch, so the name is intentional; a name split ("the built-in model")
   would contradict it. Flagged only.
4. **`ChunkSession`/`transcribe*` became `Runtime`-generic.** They now need an `AppHandle` (for the
   fallback event), and the existing chunk tests run on `tauri::test::MockRuntime`. `commands.rs` pins
   `ChunkSession<tauri::Wry>`. `transcribe_chunked` call sites in `commands.rs` are the only changes there.
5. **Clip recipe.** `say` quantises `-r`, and reusing an output path silently keeps the old file, so several
   attempts produced the same 2.54 s clip. The working recipe (fresh path each time):
   `say -v Samantha -o /tmp/st-3c.wav --data-format=LEI16@16000 "This is a test of the SpeakType speech service, testing."`
   then trim leading/trailing samples below ±200 (50 ms guard) in Python `wave`. Result: 3.295 s,
   16 kHz, mono, s16. Without trimming, 0.4 s of leading silence plus the default leading silence made the
   clip land at 5.2 s.
6. **`stt_list_models` builds the provider before reading a key.** Listing does not need audio; using the
   configured provider means a Custom endpoint with no key yet returns `NotConfigured` ("Choose a speech
   service…"), which is the Providers page's "Connect" step's own error path. `stt_test` goes through the
   same `run_provider` as dictation, so Test measures what dictation will do (including fallback).
7. **`Settings.stt_provider` is `Option<ProviderConfig>`** so a missing key round-trips as missing,
   `from_json` sees it, and the `active_provider()` accessor resolves it. After `load()` it is always
   `Some`. `bindings.ts` therefore shows `stt_provider?: ProviderConfig | null` — the Settings window keeps
   writing the old `api_url`/`model` fields, which stays harmless (migration only fills the provider when
   it is missing).
8. **Frontend untouched.** No Help/settings UI edits: the Providers page is Phase 1 (spec §6.7 D3). The
   six `stt_*` commands are deliberately not in `lib.rs`'s `FRONTEND_USED` list, which asserts what the
   webui actually calls.

## New dependencies (one-line reasons)

| Crate | Where | Reason |
|---|---|---|
| `speaktype-stt` (path) | app | The provider library Task 15 wires in. |
| `tokio-util` `0.7` | app | `CancellationToken` is required by `SttProvider::transcribe` (already in the lock via the crate). |
| `wiremock` `0.6` (dev) | app | The plan's named fallback test needs two HTTP servers (cloud closed port + local wiremock). |
| `tauri` `test` feature (dev) | app | `ChunkSession`'s existing tests need a runtime-generic handle; already a direct dependency, only a dev feature line. |
| `objc2-local-authentication` `0.3` (macOS) | app | `LAContext evaluatePolicy(.deviceOwnerAuthentication)` gates `stt_reveal_api_key` behind Touch ID / login password (review round 1, must-fix 2). |
| `block2` `0.6` (macOS) | app | Types the `evaluatePolicy:localizedReason:reply:` reply block that the LA crate's signature requires (already in the lock via objc2). |

Removed: nothing. (`transcribe.rs`'s own reqwest client is gone, but `reqwest` is still used elsewhere.)

## Open questions

1. ~~Hallucination ladder depth~~ — **closed by review round 1** (must-fix 1): `Decode` is back on
   `TranscribeRequest`, the three-pass ladder is restored exactly as on main, and `local_sends_decode_fields`
   pins temperature + `temperature_inc`.
2. ~~Per-request decode parameters for a user's own whisper.cpp server~~ — **closed by review round 1**:
   `temperature` and `temperature_inc` (0.2/0.0) are sent again, as on main. `no_speech_thold`,
   `entropy_thold` and `suppress_nst` keep the crate's fixed values from Task 14 (they did not vary per
   pass on main either).
3. **`stt_reveal_api_key` returns the key to the webview.** Now gated by LocalAuthentication before the
   Keychain read (round 1, must-fix 2), but the secret still reaches JS in clear once authenticated. If the
   reviewers want it never to leave Rust, the eye toggle would have to show only a "revealed" confirmation
   instead. Flagged; implemented per the plan's `-> String`.
4. **`llm.rs` still reads `settings.scribe_llm_api_key`** (now always blanked), so the AI-writing path would
   need `KeychainStore::get("llm.api_key")`. `llm.rs` is unowned here and dead-compiled (spec §10); its owner
   should switch it to the Keychain in the same change that revives it.
5. **`stt_set_provider` writes only the provider**, not `api_url`/`model`. `Local{model}` keeps its own
   model name while `server.rs` starts `settings.model`; they can drift. Phase 1's Providers page should
   set both together (or `Local` should drop its model field). Not decided here.

Screens: none — F4b is backend/IPC only.

## Review round 1

Commit `fix(stt): restore decode ladder; authenticate key reveal (review round 1)`.

**Must-fix 1 — decode parameters.** Crate: `pub struct Decode { temperature: f32, internal_fallback: bool }`
(`Default { 0.0, false }`), `TranscribeRequest.decode: Decode`; `local.rs` sends `temperature` and
`temperature_inc` (0.2 when `internal_fallback`, else 0.0) and `openai.rs` sends `temperature`; Deepgram is
unchanged (ignores it). App: `Attempt { temperature, use_prompt, allow_internal_fallback }` and the ladder
`[(0.0, prompt, no fallback), (0.0, no prompt, no fallback), (0.4, no prompt, +0.2 fallback)]` are restored
verbatim from main, and `debug::PassEvent.temperature` reports each pass's real temperature again.
TDD: `local_sends_decode_fields` (wiremock asserts `temperature=0.4` and `temperature_inc=0.0` — RED before
`local.rs` was touched, GREEN after) and `retry_ladder_is_the_three_passes_from_main` (pins the ladder and
each pass's `Decode`).

**Must-fix 2 — authenticated reveal.** `stt_reveal_api_key` calls `confirm_owner(&app)` first: one small
macOS-only function that starts `LAContext::evaluatePolicy(.deviceOwnerAuthentication, "Show your API key")`
on the main thread (`AppHandle::run_on_main_thread`, since the prompt is UI), with a reply block that posts
its verdict over a channel; `canEvaluatePolicy` failure, cancel, any other LA error, a main-thread failure
and a 120 s timeout all return `IpcError { kind: "authCancelled", message }`. Non-macOS builds return
`authCancelled` too. The Keychain is only read after `confirm_owner` succeeds.
Not unit-tested: the prompt is modal UI that needs a real user (and a signed bundle's LA entitlement), so a
test would either hang on a CI prompt or assert nothing; the function's contract is covered by the
`authCancelled` kind and is exercised by hand on macOS. `bindings.ts` regenerated (the command gained the
`app` handle; the generated signature is unchanged).

Commands: `cargo test -p speaktype-stt` → `ok. 7` + `ok. 15` (RED first: `local_sends_decode_fields …
FAILED`); `cargo test --workspace` → `ok. 44` (app), `ok. 15` (stt), others unchanged; `make verify` →
`EXIT=0` under load 50+ (44/3/27/3/7/15 cargo, `Tests 56`, `Tests 31`, visual `33 passed (20.7s)`).
