# SpeakType Launch — Program Plan

> **For agentic workers:** this is the program-level plan: architecture calls, libraries, team, handoff and review rules. Each phase has its own task-level plan (Phase 0: `2026-11-18-phase0-foundation.md`). Executors read the spec, this file, and their phase plan. Steps in phase plans use checkbox (`- [ ]`) syntax.

**Goal:** Ship the launch scope in the spec (Show & Tell sessions, polished dictation, rebuilt onboarding/Settings/History, updater, website) on a real design system, without lag, leaks or stale UI.

**Architecture:** Same SpeakType app and repo, new UI (strangler rebuild). Shared logic moves into libraries: a Svelte design-system package, generated typed IPC, and three Rust crates. The Tauri app becomes a thin shell of windows, commands and events.

**Tech Stack:** Tauri 2, Rust 1.93.1, Svelte 5 (runes), Vite 6, bits-ui 2, lucide, SQLite (rusqlite), ScreenCaptureKit, whisper.cpp sidecar, tauri-specta, Playwright, Vitest.

**Spec:** `docs/superpowers/specs/2026-11-18-speaktype-launch-design.md` (approved 2026-11-18). Visual reference: the SpeakType Launch Designs canvas, copied into `docs/design/boards/` with PNG renders in `docs/design/reference/` (Phase 0, Task 0).

## Global Constraints

Every task in every phase includes these. Values are verbatim from the spec.

- macOS 14+, Apple Silicon first. Keep Tauri 2 + Rust, Svelte 5 + Vite, bits-ui, lucide.
- Palette, dark: window #1E1E1E, groups #2A2A2C, controls #3A3A3C, actions/focus #0A84FF, ink/live only #EF643C, warnings #FF9F0A. Never yellow except highlighter ink.
- Focus ring: `0 0 0 1.5px rgba(10,132,255,.85)`, hugging the control, `:focus-visible` only.
- Popover elevation: `0 0 0 .5px rgba(255,255,255,.14), 0 0 0 1px rgba(0,0,0,.55), 0 4px 12px rgba(0,0,0,.28)`.
- Glows:
  - None anywhere except the widget/HUD halo `0 0 28px <state>/.34, 0 10px 26px rgba(0,0,0,.16)`.
  - The warning halo uses `rgb(176,104,36)/.5`.
- Every floating layer goes through a Portal at the window root, on the z-scale `--z-sheet < --z-popover < --z-tooltip < --z-toast`.
- Controls and rows:
  - Every control is a capsule: 28 pt in rows, 32 pt in panels and toolbars.
  - Row heights: info 36, control 44–50, icon or two-line ≈ 64.
  - Sidebar: 17 pt labels, 36 pt rows, 24 pt tiles, 256 pt wide.
  - Sheets: 28 pt radius, window dimmed ≈ 40%.
- Motion: 120–200 ms ease-out for UI, 250 ms for the ink fade. Honour `prefers-reduced-motion` (cross-fade instead of slide). No layout jank.
- Copy:
  - Sentence case, no jargon on default paths.
  - "Return", never "Enter".
  - Modifier order ⌃ ⌥ ⇧ ⌘.
  - Destructive: red label, never the default, "Delete" vs "Remove".
  - Tooltips only on text that is actually truncated, plus icon-only buttons.
- Privacy: no telemetry, nothing leaves the Mac unless configured. API keys only in the Keychain. Reports only on Send.
- Screenshots go into prompts as images, never as OCR text. Images ≤ 1568 px long edge, PNG. Assembly is deterministic.
- Dictation never pastes during a session.

## Lean code rules (owner: "architecturally best, no bloat, no AI slop")
Every reviewer rejects a change that breaks one of these:
- Build only what a board or spec line needs now. No speculative props, variants, options, config flags, or "future-proof" layers (YAGNI).
- One abstraction per real need: no wrapper with a single caller, no factory/manager/registry for one thing, no re-export chains. Use bits-ui and the platform directly before writing a helper.
- Small focused files (aim < 200 lines for a component, < 400 for a Rust module); one responsibility each.
- No comments that restate the code, no banner comments, no TODOs without an owner task; doc comments only on public library APIs and only where the name isn't enough.
- No defensive noise: no try/catch or `unwrap_or_default` that hides a failure; no duplicated validation; errors carry a user message or propagate.
- No dead code: migrated screens delete their old code in the same PR; unused exports, props, CSS and deps are removed.
- New dependency only with a one-line reason in the report; prefer none.
- Size budget: `@speaktype/ui` ≤ 60 KB gzipped JS for all components; each migrated page shrinks or holds the webui bundle size (reported in the PR).

## Review Focus (program-wide)

These are failure modes the spec implies but no feature test naturally hits. Each one is pinned by a test in the task that owns the code; the Phase 0 plan places the first five.

1. **Listener leaks.** The Settings window is hidden, not destroyed, and components remount. Every `listen()` must be released, and remounting 200× must leave zero live listeners and no duplicate handlers.
2. **Stale data across windows.** A change made in one window (model switched from the widget's right-click, a history entry deleted in the review panel) must show in every other open window within one event, without reopening.
3. **Floating layers at edges and inside sheets.** A Select whose selected item is near the bottom of the list, opened near the bottom of the window or inside a sheet, must stay fully visible, above the sheet, and never clipped.
4. **Hostile text.** Transcripts of 10k characters, CJK and RTL text, emoji, 300-character file paths: rows truncate (paths in the middle), tooltips appear only when truncated, and the layout never overflows.
5. **Rapid interaction.** Opening and closing a sheet or popover 10× within 1 s, or pressing the hotkey twice in 50 ms, must leave no stuck overlay, no orphaned focus trap, and the state machine where the user expects it.
6. **(Rust)** `session_stop` arriving while a segment is still transcribing, or while a mark is compositing, must lose no item, and Stop → prompt ready must stay under 300 ms.
7. **(Rust)** A cloud provider that returns 401, 429 or 5xx, or hangs, must produce one plain-language error, retry only when the failure is transient, and never block dictation for longer than the timeout.

---

## 1. Decision: same app, new UI (not a fresh repo)

We keep SpeakType and rebuild its UI in place.

**Keep the backend.** It already solves the hard, tested native problems: terminal-aware paste, the download queue with retry and resume, the hotkey tap, server lifecycle, notarization scripts, and the debug records. Throwing those away would cost weeks and re-open solved bugs.

**Replace the frontend surface by surface.** Each surface is rebuilt on the new design system: widget → onboarding → main window pages → review panel. `settings.css` and `ModelGrid.svelte` shrink to zero as pages migrate, and each migrated page deletes its old code in the same PR. No legacy UI survives launch.

**Reshape the Rust side into libraries.** Logic that doesn't need Tauri moves into crates with their own tests. `commands.rs`, at 2089 lines today, splits by domain.

## 2. Libraries (the "proper libraries" requirement)

| Library | Path | Responsibility | Rules |
|---|---|---|---|
| `@speaktype/ui` | `packages/ui` | Tokens, motion, every component on the Components board, `ui-gallery`, visual tests | Holds no app state and never touches Tauri. Built on bits-ui primitives. Every component works in both themes, is keyboard-operable and labelled for screen readers. The webui and `apps/site` both consume it. |
| Typed IPC | `apps/widget/webui/src/lib/bindings.ts` (generated) | Commands and events, generated from Rust with `tauri-specta` | The single source of truth is the Rust signatures. CI fails if the generated file is stale. Hand-written `ipc.ts` shrinks to re-exports, then is deleted. |
| Data layer | `apps/widget/webui/src/lib/data/` | `useEvent` and `liveQuery`: event-driven, auto-cleaned, de-duplicated fetches | The only place allowed to call `listen()`, enforced by ESLint `no-restricted-imports`. Fixes Review Focus 1 and 2 by construction. |
| `speaktype-session` | `crates/speaktype-session` | Session types, SQLite store and migrations, prompt assembly, bundle writer | Pure Rust with no Tauri dependency. Assembly has golden-file tests. |
| `speaktype-stt` | `crates/speaktype-stt` | The `SttProvider` trait (local whisper server, OpenAI-compatible, Deepgram), model catalog, retry/timeout policy, Keychain secrets | No Tauri dependency. HTTP tests use `wiremock`. |
| `speaktype-capture` | `crates/speaktype-capture` | Ink vector model, compositing (tiny-skia), crop and downscale, ScreenCaptureKit freeze-frame and window lookup (macOS module) | Pure compositing is testable on any OS, with golden PNGs. |

**AI inference practices, encoded in `speaktype-stt` and the narration engine:**

- **Provider choice:** one explicit provider setting; never guess from URL substrings.
- **Audio format:** convert to 16 kHz mono 16-bit WAV once, at capture.
- **Timeouts:** scale with audio length, `max(10 s, 3 × audio duration)`, capped at 120 s.
- **Retries:** only on connect errors, 408, 429 (honouring `Retry-After`) and 5xx. Exponential backoff (0.5 s, 1 s, 2 s) with ±20% jitter, at most 3 attempts. Never retry 400, 401 or 403.
- **Cancellation:** requests take a cancellation token, so Stop or Cancel aborts in-flight HTTP.
- **Concurrency:** the local whisper-server is serial, so at most 1 request is in flight to it. Narration segments queue with a bound of 32; when full, adjacent segments are merged rather than dropped.
- **Model lifecycle:** one model per server process. Warm-up on switch is health-checked with the existing 60 s timeout. Benchmark numbers are measured on this Mac, never claimed.
- **Prompts:** screenshots are images, not OCR text, and assembly is deterministic. No LLM in the prompt path.
- **Quality gate:** a WER regression harness (`crates/speaktype-stt/tests/wer.rs`, ignored by default, run by `make eval`) runs fixture clips through `base`. It reports word error rate (WER) and real-time factor, so model or VAD changes are measured, not felt.

**Performance budgets.** Each is asserted by a test or a measured log line in the owning phase:

| Budget | Limit |
|---|---|
| Hotkey → recording pill | < 100 ms |
| Draw-key → freeze-frame | < 50 ms |
| Stop → prompt ready | < 300 ms |
| Overlay idle CPU | ≈ 0% (render only when dirty) |
| Main window, idle | ≤ 150 MB RSS after 1 h with 1,000 history entries (virtualized lists) |
| Widget | No growth across 500 dictations (soak test, Phase 1) |

## 3. Team, models, jobs

These come from `GET https://litellm.ecouncil.ae/v1/models` and `/model/info`, checked on 2026-11-18.

| Role | Model (opencode id) | Why | Jobs |
|---|---|---|---|
| **Design-system builder** | Claude Sonnet 5.5 (Claude Code Agent, own worktree) | Owner's choice; strong at clean Svelte/CSS, sees images, tight review loop with me | `@speaktype/ui` (Phase 0 F2) — built first, before any screen work |
| **UI builder** | `litellm-ecouncil/moonshotai/Kimi-K3.1-Pro` | Strongest LiteLLM model; vision, so it can compare its own screenshots against the board PNGs; 262k context | Every Svelte surface on top of `@speaktype/ui` (widget, onboarding, main window, review, overlay canvas), website, data layer |
| **Systems builder** | `litellm-ecouncil/zai-org/GLM-5.3` | Strongest code reasoning, 460k context; no vision needed for Rust | Crates, Tauri shell, overlay/NSPanel/event tap, ScreenCaptureKit, narration/VAD, providers, updater |
| **Tester / first-pass reviewer** | `litellm-ecouncil/deepSeek-V4.1-flash` | Cheap, vision, 460k context | Writes the Review Focus tests, runs suites, first visual diff pass, reproduces bugs |
| **Ops** | `litellm-ecouncil/zai-org/GLM-5.3-Flash` | Cheap and fast; enough for scripts | Worktrees, CI workflow, release and signing scripts, merges I approve |
| **Researcher** | `litellm-ecouncil/deepSeek-V4.1-flash` | Long context, cheap | Spike research (CGEventTap and Input Monitoring, SCK exclusion APIs) |
| **Final reviewer and merger** | Claude (this session) | — | Code review plus screenshot review against the boards; merges to `main` |

The opencode orchestrator (GLM-5.3) is not used for this build: I hand off directly, so briefs aren't rewritten by a second planner.

## 4. Handoff protocol

1. **One workstream = one branch = one git worktree.**
   - Branch `ws/<id>-<slug>` from current `main`.
   - Worktree at `../speaktype-ws/<id>`.
   - Ops creates it: `git worktree add ../speaktype-ws/<id> -b ws/<id>-<slug> main`.
2. **The brief is a file, not a chat message.** It lives at `docs/superpowers/briefs/<id>.md` and is committed on the branch first. It contains:
   - Goal.
   - The spec sections it implements.
   - Phase-plan tasks, copied verbatim.
   - **Owned paths** (may edit) and **read-only paths**.
   - The Interfaces it consumes and produces.
   - The board PNGs to match.
   - Done criteria.
   - The report format.
3. **Visible launch, per `~/.config/opencode/AGENTS.md`.** Run `~/.local/share/agent-hq/dash-up "$PWD"` first, then:

   ```bash
   tmux new-window -t team -n <id> -c ../speaktype-ws/<id> \
     "opencode run --agent builder -m <model> --title '<id> <slug>' --auto \
      -f docs/superpowers/briefs/<id>.md \
      'Execute the attached brief task by task with TDD. Commit after each task. Finish by writing .handoff/<id>/REPORT.md.'"
   ```

   At most 4 builders run at once (6 panes max, minus SHARED SHELL and the tester).
4. **The report** at `.handoff/<id>/REPORT.md` contains:
   - Tasks done.
   - Exact commands run, with tails of their output.
   - Deviations from the brief and why.
   - Open questions.
   - Screenshot paths: `.handoff/<id>/screens/<name>-{light,dark}.png`, from the visual pipeline, at 2×.

   `.handoff/` is git-ignored except `REPORT.md`.
5. **Builders never edit outside their owned paths.** If they need a contract change, they stop and write it under "Open questions" in the report. I make the contract change on `main`, and they rebase.
6. **Change requests** go back to the same session: `opencode run -s <session-id> -f review/<id>-round<n>.md 'Address this review.'`. Review rounds are committed on the branch.

## 5. Review gate and merge policy

A workstream merges only after all of these pass.

**Automated, in the worktree** (`make verify`, added in Phase 0):

- `npm run check`, `npm test` and `npm run lint` (webui plus `@speaktype/ui`).
- `npm run visual`: Playwright screenshots match baselines, threshold 0.1%.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
- The bindings are fresh: `cargo test -p speaktype export_bindings && git diff --exit-code apps/widget/webui/src/lib/bindings.ts`.

**First pass (tester, DeepSeek):** runs `make verify` from scratch, checks that the Review Focus tests exist and fail without the fix, and compares screens to the board PNGs. Writes `review/<id>-firstpass.md`.

**Code review (me):**

- Correctness, and every spec acceptance line for the workstream ticked with evidence.
- Leaks: every `useEvent` or timer released; Rust tasks joined or cancelled.
- Staleness: every view backed by a `liveQuery` with the right invalidating events.
- No hand-written IPC.
- No new dependency without a reason line in the report.
- File ownership respected.

**Screenshot review (me):** every screen in both themes, compared against its board PNG for:

- Spacing, row heights and type sizes (per Global Constraints).
- Alignment columns and focus ring.
- Floating layers.
- Truncation.
- Empty, loading and error states present.
- Motion checked with a short screen recording (`.handoff/<id>/motion/*.mov`) for sheets, steps and HUD morphs.

**Merge:**

- Rebase on `main`, then `git merge --no-ff ws/<id>-<slug>`.
- Commit messages end with the attribution line.
- Delete the worktree.
- Re-baseline only the visual baselines the workstream owns.
- I merge to local `main` as soon as review passes. **Pushing to `origin` happens only after you confirm** (outward-facing).

## 6. Phase map

Each phase gets its own task-level plan, written when the phase starts against the then-real interfaces. Phase 0 is written now.

| Phase | Workstreams (owner model) | Depends on | Plan |
|---|---|---|---|
| **0 Foundation** | T0 design refs + repo hygiene (Claude) · F1 land Scribe checkpoint (Claude) · **F2 `@speaktype/ui` first** (Sonnet) · then F3 Rust workspace + `speaktype-session` + typed IPC (GLM) · F4 `speaktype-stt` providers + Keychain (GLM) · F5 data layer (Kimi, after F3 bindings) | — | `2026-11-18-phase0-foundation.md` |
| **1 Core loop** | D1 widget/HUD shell + paste recovery + mic picker (Kimi + GLM for Esc/mic fallback) · D2 onboarding 1–6 (Kimi) · D3 main window shell, sidebar, search, General/Appearance/Dictation/Providers/Permissions/Storage/Advanced pages + Speech models sheet (Kimi) | F2, F3, F4, F5 | written at Phase 0 merge |
| **2 Show & Tell** | S1 overlay windows + draw-key tap + ink canvas (GLM + Kimi for canvas) · S2 narration + capture + compositing (GLM) · S3 HUD session mode + captions (Kimi) | F3, D1 | after S1 spikes (spec §9, items 1–4) |
| **3 Review & History** | L0 quick review panel · L1 History page + Details + Report a problem · L2 Timeline editing · L3 Result view, bundle, copy/export | F3, D3, S3 | at Phase 2 merge |
| **4 Ship** | R1 updater + signing + release + update UI · R2 website `apps/site` · R3 report endpoint (serverless, retention policy) | F2 | can start after Phase 0 |

**Spikes run first in Phase 2** (researcher plus GLM, 1 day, throwaway code on `spike/*` branches, never merged). Spec §9 items 1–4: draw-key tap under Accessibility alone, overlay over full-screen apps and Spaces, SCK exclusion with < 50 ms freeze-frame, `ignoresMouseEvents` flip latency. Their findings amend the Phase 2 plan before any S-workstream starts.

**Cut order if time runs short** (from spec §10): `.docx` export → Timeline split → chat target / Copy images → website changelog → drag reorder. Never cut: overlay, narration, Prompt copy, onboarding, updater, signing.

## 7. What I decided (for your check)

- **Same app, new UI**, rather than a fresh project (§1).
- **New libraries:** a `packages/ui` npm workspace, three Rust crates in a root Cargo workspace, and generated IPC via `tauri-specta`, which replaces hand-written `ipc.ts`.
- **New-install "After pasting" default** is **Nothing**. Existing users keep Return. Recorded because spec §12 left it open.
- **Pushes:**
  - Merges go to local `main`.
  - Each push to `origin` waits for your yes.
  - Release builds and notarization also wait for your yes.
- **The report endpoint (R3)** needs a hosting choice and a retention policy from you before it starts. Phase 4 blocks on that, nothing earlier does.
