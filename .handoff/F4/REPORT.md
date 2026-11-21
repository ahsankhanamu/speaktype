# F4 — `speaktype-stt` providers (Phase 0, Tasks 13–14)

Branch `ws/F4-stt`, base `c33853c`. Owner model GLM-5.3. Task 15 (app wiring) intentionally not
started — it waits on F3's typed-IPC merge, per the brief.

## Tasks done

- **Task 13 — provider config, errors, keychain secrets** — commit `1f74195`
  `feat(stt): provider config, errors, keychain secrets`.
  `crates/speaktype-stt/{Cargo.toml,src/{lib,config,secrets,error}.rs}` + the
  `"crates/speaktype-stt"` member line in root `Cargo.toml` (needed for `cargo test -p
  speaktype-stt`). 7 unit tests, all plan assertions (serde round-trips per variant,
  `default_base_url` values, MemoryStore set/get/delete, `is_transient` truth table, exact
  `user_message` copy for "Groq").
- **Task 14 — providers, retry and timeouts** — commit `2719b3f`
  `feat(stt): local, OpenAI-compatible and Deepgram providers with retry/timeout/cancel`.
  `src/{provider,local,openai,deepgram,retry}.rs`, `tests/providers.rs` (wiremock). All 8
  plan-named tests plus 3 extra (below), 12/12 green, stable across 3 repeated runs (~1.4 s each).

## Commands run (tails)

```
$ npm ci                                          # worktree setup, per brief
npm warn install-scripts 2 packages have install scripts not yet covered by allowScripts:
npm warn install-scripts   esbuild@0.25.12 (postinstall: node install.js)
npm warn install-scripts   fsevents@2.3.3 (install scripts present)

$ cargo test -p speaktype-stt                      # Task 13 RED (types absent)
error: could not compile `speaktype-stt` (lib test) due to 42 previous errors; 3 warnings emitted

$ cargo test -p speaktype-stt                      # Task 13 GREEN
running 7 tests
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test -p speaktype-stt --test providers     # Task 14 RED
error[E0432]: unresolved imports `speaktype_stt::provider_for`, `speaktype_stt::timeout_for`,
               `speaktype_stt::TranscribeRequest`
error[E0405]: cannot find trait `SttProvider` in crate `speaktype_stt`

$ cargo test -p speaktype-stt                      # Task 14 GREEN (7 lib + 12 providers)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.49s
# repeated ×3 for the timing-sensitive tests: 1.49s / 1.53s / 1.39s, 12/12 each

$ cargo clippy -p speaktype-stt --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)
$ cargo fmt --all --check && echo FMT-OK
FMT-OK

$ cargo test -p speaktype-stt --test providers never_waits   # review round 1 RED
thread 'never_waits_longer_than_10s_for_retry_after' panicked:
transcribe must return instead of waiting out a huge Retry-After: Elapsed(())
test result: FAILED. 0 passed; 1 failed; ... finished in 5.01s

$ cargo test -p speaktype-stt --test providers never_waits   # review round 1 GREEN
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.00s
# whole-crate suite after the fix: 7 lib + 13 providers, all green
```

`make verify` environment notes (worktree bootstrap, not code): a fresh worktree is missing
three local artifacts the gate needs, each fixed by the setup the CI workflow already performs.
Run 1 failed with `resource path 'binaries/whisper-server-aarch64-apple-darwin' doesn't exist`
(tauri-build; the sidecar is gitignored) — fixed by copying it from the main checkout. Run 2
failed with `The 'frontendDist' configuration is set to "../../../apps/widget/webui/dist" but
this path doesn't exist` (`tauri::generate_context!`) — fixed with
`npm --prefix apps/widget/webui run build`. Run 3 (pre-review-fix) was superseded by the code
change. Run 4 failed only in the visual suite: `Executable doesn't exist at …/
chromium_headless_shell-1255/…` — the repo's Playwright pins revision 1255 while the cache had
older ones; fixed with `npx -w @speaktype/ui playwright install chromium` (the step
`.github/workflows/build.yml` runs before `make verify`). Run 5 is the real gate:

```
$ make verify
# cargo fmt --all --check            ✓
# cargo clippy --workspace -D warnings ✓
# cargo test --workspace:  speaktype 38 passed · speaktype-stt lib 7 passed ·
#                          speaktype-stt providers 13 passed (2.18s) (+ 3 empty doc-test lines)
# npm run -ws --if-present check:    svelte-check 0 errors ×2
# npm run -ws --if-present test:      webui 56 passed · @speaktype/ui 31 passed
# npm -w @speaktype/ui run visual:
  33 passed (21.1s)
EXIT=0
```

## Deviations from the plan (all ledgered in `.superpowers/sdd/…/progress.md`)

1. **Test timing strategy differs from the reviewer's note.** The note says "use
   `tokio::time::pause` for the timeout and Retry-After tests". Pause **is** used for the
   timeout/cancel tests, but tokio 1.49's auto-advance is incompatible with wiremock round-trips:
   in `runtime/time/mod.rs park_thread_timeout`, a parked runtime advances the clock to the next
   timer whenever `did_wake` is false — and `did_wake` is only set by cross-thread unparks
   (`current_thread`'s `Wake::wake_by_ref`), not by in-thread I/O wakes. So the per-attempt 10 s
   timeout fired mid-round-trip (observed: `Err(Timeout)` instead of 503/429). Fix:
   `retries_503_then_succeeds` and `honours_retry_after_on_429` run in **real time** (~1.5 s and
   ~1.0 s wall, deterministic — the Retry-After header makes the wait exactly 1 s);
   `times_out_and_cancels` and `cancels_in_flight_request` keep `start_paused` but hang the
   request on a **bound-never-accepting `TcpListener`** (deterministic `Err(Timeout)` after 3
   attempts / `Err(Cancelled)` at 100 ms, in milliseconds of wall time). Cost: suite takes ~2.5 s
   real instead of ~0.1 s.
2. **JSON shapes made fully camelCase** (variant fields `baseUrl`, `fallbackToLocal`; service
   names `openai|groq|deepgram|custom`): the plan pins only the enum-level
   `#[serde(tag = "kind", rename_all = "camelCase")]`, but §7.4 makes every IPC payload
   camelCase and Task 15 sends these types across IPC. Pinned by round-trip tests.
3. **`user_message(Cancelled)` = "Cancelled."** — the plan's copy table omits Cancelled and the
   method must be total.
4. **`CloudService::secret_account()`** ("stt.openai"…) added beyond the listed interface — the
   plan's `KeychainStore` comment pins account naming `stt.<service>`, and Task 14's providers
   and Task 15's key commands both need it in one place.
5. **`Local` has no URL in the config**, so the crate pins `LOCAL_ENDPOINT =
   "http://127.0.0.1:8002/inference"` (mirrors the `settings.rs` default). This is safe because
   `server.rs` rewrites `settings.api_url` to the live sidecar port, and Task 15 maps any
   non-default `api_url` to `Server { base_url }`.
6. **Local/Server port is first-pass only**: `TranscribeRequest` (plan-pinned) has no
   temperature/`temperature_inc`/`use_prompt` fields, so `local.rs` ports the `FIRST_PASS`
   constants (`temperature=0`, `temperature_inc=0.0`) and the app's hallucination retry ladder
   cannot be expressed through this API (open question 1).
7. **OpenAI-compatible sends `prompt` when set** — not in the plan's field list, but the old
   `transcribe.rs` passed prompt on the OpenAI-compat path and OpenAI documents the field.
   `temperature` is not sent (plan's field list omits it; OpenAI's default is 0 = old behaviour).
8. **`SecretError` during `provider_for` construction → `NotConfigured`** — the plan's `SttError`
   has no secret variant; a missing key is the common case. Cost: keychain *access* failure
   detail is lost (open question 2).
9. **Empty cloud `base_url` resolves to `default_base_url()`** (Custom with no address →
   `NotConfigured`); **408 → `Timeout`**; `Retry-After` parsed as integer seconds only
   (HTTP-date falls back to the jittered backoff).
10. **`WhisperServer` carries an optional model** so `list_models` returns the sidecar's model
    (Local) or an empty list (Server — whisper.cpp servers have no listing API) instead of
    erroring, keeping the trait uniform for the Task 15 picker. `list_models` runs under the same
    retry policy with `timeout_for(0)`'s 10 s floor.
11. **Three tests beyond the plan's list** pin plan-specified behaviour with no named test:
    curated 404 fallback (Groq + OpenAi), Custom 404 → `Err(Server{404})`, Deepgram's curated
    model list.
12. **`specta = "=2.0.0-rc.28"`** — exact pin matching F3's `speaktype-session` (tauri-specta
    requires precisely this line; two specta versions in one workspace would break the Task 15
    IPC export).
13. **`KeychainStore::delete` maps keyring `NoEntry` → `Ok(())`** (idempotent delete) — verified
    against keyring 3.6.3's macOS backend, which returns `NoEntry` for missing items, so the arm
    is required, not defensive.
14. Test WAV payload is an ASCII stand-in — nothing on either side of those tests decodes audio,
    and it keeps multipart bodies valid UTF-8 for wiremock's `body_string_contains`.

## New dependencies

`crates/speaktype-stt/Cargo.toml` (all from the plan's Task 13 dep list except where noted):

- `reqwest` 0.12, `default-features = false`, `json`+`multipart`+`rustls-tls` — HTTP transport;
  rustls per the brief (no native-tls anywhere in the crate's tree).
- `tokio` 1, `time`+`macros` — per-attempt `timeout`, backoff `sleep`, `select!` cancellation.
- `tokio-util` 0.7 — `CancellationToken`.
- `async-trait` 0.1 — async methods on the `SttProvider` trait object.
- `thiserror` 2 — `SttError`/`SecretError` Display + Error.
- `serde`/`serde_json` 1 — config (de)serialization, response parsing (serde_json also runs the
  round-trip tests; not named in the plan's dep list, required by both).
- `url` 2 — safe base+path joining for cloud endpoints.
- `bytes` 1 — `TranscribeRequest.wav` (plan-specified).
- `keyring` 3, `apple-native` — macOS Keychain, native backend only per the brief.
- `specta` `=2.0.0-rc.28`, `derive` — `ProviderConfig`/`CloudService`/`RemoteModel` cross the
  typed IPC in Task 15.
- Dev: `wiremock` 0.6 — provider HTTP tests; `tokio` dev adds `rt`+`macros`+`test-util`
  (`start_paused`) + `net` (the dead-listener endpoint).

## Review round 1 (reviewer: session reviewer)

- **Must-fix (fixed): Retry-After was honoured unbounded** — a server sending `Retry-After: 3600`
  would have blocked dictation for an hour, violating program plan Review Focus 7 ("never block
  dictation for longer than the timeout"). `retry.rs` now honours `Retry-After` only up to
  `RETRY_AFTER_MAX_WAIT_MS = 10_000` (the timeout floor); a longer hint returns the
  `RateLimited` error immediately without retrying. Pinned by
  `never_waits_longer_than_10s_for_retry_after` (429 + `Retry-After: 3600` → `Err(RateLimited
  { retry_after_ms: Some(3_600_000) })`, exactly 1 request via `expect(1)`, wall time < 1 s;
  the test wraps the call in a 5 s bound so a regression fails in seconds instead of hanging
  the suite for the hour). RED observed at 5.01 s, GREEN after the fix.
- **Do not commit `.superpowers/`** — complied; the ledger stays untracked (it was never
  committed on this branch).
- Committed together with this report as
  `fix(stt): cap Retry-After wait (review round 1)`.

## Open questions

1. **Task 15 contract gap — hallucination ladder.** `transcribe_verified`'s retry ladder
   (temperature 0 → drop prompt → temperature 0.4 with internal fallback) needs per-attempt
   decode parameters, but the plan-pinned `TranscribeRequest` has no fields for them. Either the
   crate interface gains (internal) decode knobs that `transcribe.rs`'s adapter passes per pass,
   or the ladder moves/dies in the app layer. Needs a ruling before Task 15.
2. **Keychain access failures lose detail** (mapping 8): if `SecretStore::get` fails for a
   non-missing reason, `provider_for` returns `NotConfigured`. Task 15's `stt_*` commands may
   want to pre-flight the keychain and surface `SecretError`'s message instead.
3. **`Server` `list_models` returns an empty list** (a user's whisper.cpp server has no listing
   API) — the Providers/speech-model picker must not render that as "no models configured".
4. **Deepgram carries no per-segment quality metrics** under the plan's request (no
   `utterances` param): `Transcript.segments` is always empty there, so app-side hallucination
   scoring runs text-only (blocklist heuristics) for Deepgram. Acceptable?
5. **WER harness** (`crates/speaktype-stt/tests/wer.rs`, `make eval`) is named in program plan §2
   but not in Phase 0 Tasks 13–15 — which workstream owns it, and when?
6. **`make verify` on a fresh worktree** needs two gitignored artifacts (`binaries/whisper-server-*`,
   `webui/dist`) that CI builds per its own flow. Ops may want `make verify` to check for them
   with a plain error message (I copied/built them locally).
7. **Root `Cargo.toml` members**: F3 will add its own two crate lines next to mine — expect a
   trivial merge conflict at merge time.

## Screens

None — this workstream owns no UI.
