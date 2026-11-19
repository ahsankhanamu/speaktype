# Common rules for every workstream brief

**Read first:**
1. `docs/superpowers/plans/2026-11-18-speaktype-launch-program.md`, specifically Global Constraints, Lean code rules, and §4–5 (handoff, review gate).
2. Your tasks in `docs/superpowers/plans/2026-11-18-phase0-foundation.md`. Follow them exactly: file names, signatures, test names, values.
3. The spec sections your tasks cite, in `docs/superpowers/specs/2026-11-18-speaktype-launch-design.md`.

**Fresh worktree setup** (run once, from the worktree root; both artifacts are git-ignored):
```bash
export CARGO_TARGET_DIR=/Users/ahsan/Desktop/speaktype-ws/.target   # shared build output: one ~10 GB tree for all worktrees (disk is tight); keep it exported for every cargo/make call
npm ci
npm --prefix apps/widget/webui run build
mkdir -p apps/widget/src-tauri/binaries && cp /Users/ahsan/Desktop/speaktype/apps/widget/src-tauri/binaries/* apps/widget/src-tauri/binaries/
```

**Work rules:**
- TDD per step:
  1. Write the failing test.
  2. Run it and see it fail.
  3. Implement.
  4. See it pass.
  5. Commit with the plan's message, ending with the line `Built-with: <your model name> (opencode)`.
- Edit only your **owned paths**. If something outside them must change, stop and write it under "Open questions" in your report.
- Lean code, which the reviewer enforces:
  - No speculative options.
  - No single-caller wrappers.
  - No comments that restate code.
  - No swallowed errors (`let _ =`, `unwrap_or_default`, empty `catch`), except where the plan says best-effort.
  - No dead code.
  - Every new dependency gets a one-line reason in the report.
- Before reporting, run `make verify` from the worktree root. It must exit 0.

**No subagents.** Don't spawn subagents or a second-model review, because review is the reviewer's job and it doubles the cost. Run tests and `make verify` yourself, in your own pane.

**Report:** write `.handoff/<WS>/REPORT.md` and commit it with `git add -f`. It contains:
- Tasks done, with commit hashes.
- The exact commands run, with the tail of each output.
- Deviations from the plan and why.
- New dependencies with reasons.
- Open questions.

Your final chat message is a 5-line summary.
