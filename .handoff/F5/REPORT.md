# F5 handoff — webui data layer

## Tasks done

- **Phase 0, Task 16 — data layer:** `29a5246bdc091eeffdc709f2f36f91a06aa192f6` (`feat(webui): event-driven data layer with leak-proof subscriptions`).
- Added `EventBus`/`setBus`, leak-safe `useEvent`, shared/ref-counted `liveQuery`, and the public `data/index.ts` surface. Production data-layer code is **161 lines**.
- Added all six required tests with real Svelte mount/unmount lifecycles, deferred listener/fetch fakes, and fake timers.
- Migrated Widget, ModelGrid, MicTest, and Settings subscriptions to `useEvent`; removed `listenEvent` from `ipc.ts`. `emitEvent` remains API-compatible as a data-bus re-export.
- Added flat ESLint config with Svelte and TypeScript parsing, raw-event restrictions, typed-event restrictions, and interval restrictions.
- No subagents were used. Final review was a separate self-review pass, as required by the brief's no-second-review rule.

## TDD evidence

1. After the test files existed but before implementation, `npm -w speaktype-webui test data` failed module resolution as expected.
2. With only compilable API scaffolding, the same command produced **6 failed / 6 required behavioral assertions**: two `useEvent` cleanup tests and four `liveQuery` sharing/race/invalidation/polling tests.
3. `npm -w speaktype-webui test data/useEvent.test.ts` then passed **2/2** with no warnings.
4. After `liveQuery` implementation, `npm -w speaktype-webui test data` passed **6/6** with no warnings.
5. The lint boundary harness failed RED for plain-JS event imports and `window.setInterval`, then passed all six cases after the config fix.

## New dependencies

All additions are development-only; there are **no new runtime dependencies**.

- `eslint@^10.15.0` — runs the required flat-config boundary checks.
- `eslint-plugin-svelte@^3.24.1` — parses Svelte files and supplies the required base config.
- `typescript-eslint@^8.73.0` — parses TypeScript and TypeScript inside Svelte scripts.
- `package-lock.json` was updated by npm for those dependencies. Both install runs reported the same pre-existing audit total: 1 moderate and 2 critical vulnerabilities; no out-of-scope audit fix was attempted.

## Commands run and output tails

### Fresh worktree setup

The assigned linked worktree already existed at `/Users/ahsan/Desktop/speaktype-ws/F5`, was clean, and was already on `ws/F5-data`; no nested worktree was created.

```sh
npm ci
# tail:
# added 153 packages, and audited 156 packages in 15s
# 3 vulnerabilities (1 moderate, 2 critical)

npm --prefix apps/widget/webui run build
# tail:
# ✓ 5221 modules transformed.
# ✓ built in 35.58s

mkdir -p apps/widget/src-tauri/binaries && cp /Users/ahsan/Desktop/speaktype/apps/widget/src-tauri/binaries/* apps/widget/src-tauri/binaries/
# exit 0; no output

npm -w speaktype-webui install --save-dev eslint eslint-plugin-svelte typescript-eslint
# tail:
# added 101 packages, and audited 257 packages in 1m
# 3 vulnerabilities (1 moderate, 2 critical)
```

### Data tests, lint, and type checks

```sh
npm -w speaktype-webui test data
# RED tail, against compilable stubs:
# Test Files  2 failed (2)
# Tests  6 failed (6)

npm -w speaktype-webui test data/useEvent.test.ts
# GREEN tail:
# Test Files  1 passed (1)
# Tests  2 passed (2)

npm -w speaktype-webui test data
# GREEN tail:
# Test Files  2 passed (2)
# Tests  6 passed (6)

npm -w speaktype-webui test data lib/ipc.test.ts
# tail:
# Test Files  3 passed (3)
# Tests  9 passed (9)

npm -w speaktype-webui run check
# tail:
# COMPLETED 5297 FILES 0 ERRORS 0 WARNINGS 0 FILES_WITH_PROBLEMS

npm -w speaktype-webui run lint
# final: exit 0; no output

zsh .handoff/F5/check-eslint-boundaries.zsh
# final output:
# PASS raw-listener-svelte: rejected
# PASS raw-listener-js: rejected
# PASS generated-events: rejected
# PASS direct-interval: rejected
# PASS member-interval: rejected
# PASS data-layer exception
```

The final six-case boundary run was preceded by a RED run whose two failures were exactly `raw-listener-js` and `member-interval`; all other cases already rejected/allowed correctly.

### Full webui suite and final verification

```sh
npm -w speaktype-webui test src/settings/Settings.test.ts
# tail:
# Test Files  1 passed (1)
# Tests  4 passed (4)

VITEST_MAX_THREADS=1 VITEST_MAX_FORKS=1 npm -w speaktype-webui test
# tail:
# Test Files  6 passed (6)
# Tests  37 passed (37)

VITEST_MAX_THREADS=1 VITEST_MAX_FORKS=1 make verify
# evidence from final log:
# cargo fmt --all --check                                      exit 0
# cargo clippy --workspace --all-targets -- -D warnings        exit 0
# cargo test --workspace: 91 passed, 0 failed, 1 ignored
# workspace checks: 0 ERRORS, 0 WARNINGS (both checked workspaces)
# workspace lint: exit 0
# webui: Test Files 6 passed; Tests 37 passed (37)
# visual: 33 passed (20.0s)
# process exit code: 0
```

`make verify` also passed once before the final two lint-boundary tightenings; the run above is the final run over the exact implementation candidate.

### Earlier full-suite failures (all named)

The default multi-worker suite was run before and after migration on a host with load average over 40. Dynamic Svelte imports delayed the existing 100 ms Settings waits beyond their configured timeouts. No assertion failed, and every case passed in isolation and in both single-worker full verification runs.

- Baseline `npm -w speaktype-webui test`: **27 passed, 4 timed out** — `mounts the settings shell and renders all tabs`, `switches tabs on sidebar click`, `toggles the input-device info popover`, `shows only the active tab's footer (per-pane footers)`.
- Post-migration default run: **34 passed, 3 timed out** — the first three names above; the per-pane footer test passed.
- Default standalone retry without parallel verification commands: **35 passed, 2 timed out** — `mounts the settings shell and renders all tabs` and `switches tabs on sidebar click`; the other two passed.
- Isolated Settings run: **4/4 passed**. Single-worker full runs: **37/37 passed**, including all four Settings tests.

Full command logs remain under `.handoff/F5/logs/` in the worktree; that directory is intentionally git-ignored.

## Deviations from the plan and why

- **Explicit refetches supersede in-flight requests.** The shared initial fetch is de-duplicated per key, as required by the two-component test. The separate race test requires a newer request to resolve before the older one, so event/manual refetches are not blocked by an older request; sequence numbers prevent stale application.
- **App-lifetime theme uses `busListen`.** `startTheme()` is called from entry modules outside Svelte component initialization, so `useEvent` would have no legal effect scope. Raw Tauri event access remains confined to `data/bus.ts`; the single app-lifetime theme listener goes through that shared bus instead.
- **`emitEvent` lives in `data/bus.ts`.** Keeping it in `ipc.ts` would conflict with the requirement to ban the whole Tauri event module outside `data/**`; the IPC API remains unchanged through a re-export.
- **Generated `bindings.ts` is ESLint-ignored.** It owns generated raw Tauri bindings and is not an F5-owned editable path.
- **`DebugTab.svelte` is temporarily ESLint-ignored.** `svelte-eslint-parser@1.10.1` rejects its valid parenthesized ternaries inside `{@const}` at EOF. This was reproduced independently; `svelte-check` accepts the file, and it was not modified.
- **Three legacy polling files have narrow interval-rule exemptions:** `Onboarding.svelte`, `Settings.svelte`, and `Widget.svelte`. F5 permits subscription-only call-site edits, not those independent polling migrations. Every other JS/TS/Svelte file rejects direct and member `setInterval` calls, and `data/**` retains the intended exception.
- **Single-worker final verification.** Existing Settings tests proved host-load-sensitive; `VITEST_MAX_THREADS=1 VITEST_MAX_FORKS=1` avoids resource starvation without changing test content, timeouts, or repository config.

## Open questions

- Which owning workstream will migrate the three legacy polling loops to `liveQuery({ pollMs })` and remove their targeted ESLint exemptions?
- Should the app-lifetime theme subscription remain on `busListen`, or should a later surface task move `startTheme()` behind one root-component lifecycle so it can use `useEvent` literally?
- Can `svelte-eslint-parser` be upgraded or patched so `DebugTab.svelte` returns to normal lint coverage?
