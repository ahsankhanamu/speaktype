# Phase 0 — Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task by task. Steps use checkbox (`- [ ]`) syntax for tracking. Read the program plan `2026-11-18-speaktype-launch-program.md` first; its Global Constraints and handoff rules apply to every task here.

**Goal:** Put the design system, typed IPC, data layer and core Rust libraries in place, so Phase 1–3 workstreams build screens and features on them in parallel.

**Architecture:**
- **Libraries.** An npm workspace adds `packages/ui` (`@speaktype/ui`). A root Cargo workspace adds `crates/speaktype-session`, `crates/speaktype-stt` and `crates/speaktype-capture` (image ops only in this phase).
- **Typed IPC.** `tauri-specta` generates `webui/src/lib/bindings.ts`.
- **Data layer.** `webui/src/lib/data/` is the only place that subscribes to backend events.
- **Scribe UI.** The unreleased Scribe UI and `*_scribe_*` commands are removed. Its capture paths write into the new session store.

**Tech Stack:** Svelte 5 runes, bits-ui 2, Vite 6, Vitest 3, Playwright, Rust 1.93.1, rusqlite 0.40 (bundled), tauri-specta 2, wiremock, keyring, image, fast_image_resize, tokio-util (CancellationToken).

**Spec:** `docs/superpowers/specs/2026-11-18-speaktype-launch-design.md`. The sections this plan touches are §3, §6.4, §6.5, §6.7 (Providers), §6.9, §7.2–7.5 and §12.

## Workstreams and file ownership

| WS | Owner model | Tasks | Owns (may edit) |
|---|---|---|---|
| T0 | Claude | 0 | `docs/design/**`, `.gitignore`, `docs/superpowers/**` |
| F1 | Claude | 1 | whole tree, fixes only; no refactors |
| F2 | Claude Sonnet 5.5 | 2–7 | `package.json` (root), `packages/ui/**`, `apps/widget/webui/src/lib/theme.css`, `apps/widget/webui/package.json` |
| F3 | GLM-5.3 | 8–12 | `Cargo.toml` (root), `rust-toolchain.toml`, `crates/speaktype-session/**`, `crates/speaktype-capture/**`, `apps/widget/src-tauri/**` except `transcribe.rs`/`settings.rs` provider fields, `apps/widget/webui/src/lib/bindings.ts`, deletion of `ScribeTab.svelte` and its import in `Settings.svelte`, `Makefile` `verify` target |
| F4 | GLM-5.3 (second pane) | 13–15 | `crates/speaktype-stt/**`, `apps/widget/src-tauri/src/transcribe.rs`, provider fields in `settings.rs` |
| F5 | Kimi-K3.1-Pro (second pane) | 16 | `apps/widget/webui/src/lib/data/**`, `apps/widget/webui/eslint.config.js` |

**Order:** T0 → F1 → **F2 (design system) first**, merged and screenshot-approved → then F3 and F4 in parallel → F5 after Task 12. (Owner: design system before anything else, so every later agent integrates finished components.)

## Review Focus

Each line is pinned by a named test in its owning task.

1. **Listener leaks on remount.** Covered by `data/useEvent.test.ts › releases every listener after 200 mount/unmount cycles` (Task 16).
2. **Stale data across windows.** Covered by `data/liveQuery.test.ts › refetches when an invalidating event arrives` and `› a slow earlier response never overwrites a newer one` (Task 16).
3. **Pop-up menu at window edges and inside a sheet.** Covered by `visual/behaviour.spec.ts › select opens item-aligned and stays inside the viewport near the bottom edge` and `› select inside a sheet renders above the sheet` (Task 7).
4. **Hostile text.** Covered by `visual/behaviour.spec.ts › long path middle-truncates and shows a tooltip only when truncated` (Task 7), plus `assemble::tests::clip_containing_backticks_uses_longer_fence` and `bundle::tests::slug_handles_unicode_and_length` (Tasks 10, 11).
5. **Rapid open/close.** Covered by `visual/behaviour.spec.ts › sheet toggled 10× in 1 s ends closed with focus restored and no inert leftovers` (Task 7).
6. **(Rust) cloud failures.** Covered by `speaktype-stt` tests `retries_503_then_succeeds`, `never_retries_401`, `honours_retry_after_on_429`, `times_out_and_cancels` (Task 14).

---

### Task 0: Design references in the repo (Claude)

**Files:**
- Create: `docs/design/boards/*.dc.html` (copied from the canvas)
- Create: `docs/design/reference/<Board>.png` (rendered at 2×)
- Create: `docs/design/README.md` (board → spec section → workstream map)
- Modify: `.gitignore` (add `.handoff/*` and `!.handoff/*/REPORT.md`)

- [ ] **Step 1:** Copy every board file from the canvas scratchpad into `docs/design/boards/`, then render each to PNG. Expected: one PNG per board, each opened and checked by eye.
- [ ] **Step 2:** Write the README table, with columns Board | Spec § | Workstream.
- [ ] **Step 3: Commit.**

```bash
git add docs/design .gitignore docs/superpowers
git commit -m "docs: launch spec, program + phase 0 plans, design references"
```

### Task 1: Land the Scribe checkpoint (F1)

**Files:** whatever fails; no new features.

- [ ] **Step 1: Run everything.**

```bash
cd apps/widget/webui && npm ci && npm run check && npm test
cd ../src-tauri && cargo clippy --all-targets -- -D warnings && cargo test
```

Expected: all pass. If not, fix with the smallest change and note each fix in the report.
- [ ] **Step 2:** Commit the staged and unstaged work on `feat/scribe`, including `lib/ui/Button.svelte`: `git add -A apps tools docs README.md .gitignore && git commit -m "feat(scribe): checkpoint before launch rebuild"`. Do not add `docs/superpowers/` here; T0 owns that.
- [ ] **Step 3:** Claude reviews, then merges: `git checkout main && git merge --no-ff feat/scribe`.

---

### Task 2: Workspace and `@speaktype/ui` tokens (F2)

**Files:**
- Create: `package.json` (root): `{"private":true,"workspaces":["packages/ui","apps/widget/webui"]}` (R2 adds `apps/site` when it exists)
- Create: `packages/ui/package.json`
  - name `@speaktype/ui`, `"type":"module"`, `"svelte":"./src/index.ts"`.
  - exports: `"."`, `"./tokens.css"`, `"./motion"`.
  - peerDeps: `svelte ^5.19`, `bits-ui ^2.19`.
- Create: `packages/ui/src/tokens/tokens.css`
- Create: `packages/ui/src/tokens/tokens.test.ts`
- Modify: `apps/widget/webui/src/lib/theme.css`
  - Import `@speaktype/ui/tokens.css`.
  - Re-point every existing variable (names kept, spec §3) to an `--st-*` token, e.g. `--accent: var(--st-accent)`.
  - Leave no literal colours.
- Modify: `apps/widget/webui/package.json`: add the dependency `"@speaktype/ui": "*"`.

**Interfaces:**
- Produces CSS custom properties on `:root`, overridden under `:root[data-theme="light"]` and under `@media (prefers-color-scheme: light) :root:not([data-theme="dark"])`. Exact names and dark values:

| Token | Dark | Light |
|---|---|---|
| `--st-window` | #1E1E1E | #F5F5F7 |
| `--st-group` | #2A2A2C | #FFFFFF |
| `--st-control` | #3A3A3C | #E5E5EA |
| `--st-text` | rgba(255,255,255,.88) | rgba(0,0,0,.88) |
| `--st-text-secondary` | rgba(255,255,255,.55) | rgba(0,0,0,.55) |
| `--st-separator` | rgba(255,255,255,.08) | rgba(0,0,0,.1) |
| `--st-accent` | #0A84FF | #007AFF |
| `--st-live` | #EF643C | #EF643C |
| `--st-warning` | #FF9F0A | #FF9500 |
| `--st-danger` | #FF453A | #FF3B30 |
| `--st-success` | #30D158 | #34C759 |
| `--st-focus-ring` | `0 0 0 1.5px rgba(10,132,255,.85)` | same |
| `--st-elevation-popover` | `0 0 0 .5px rgba(255,255,255,.14), 0 0 0 1px rgba(0,0,0,.55), 0 4px 12px rgba(0,0,0,.28)` | `0 0 0 .5px rgba(0,0,0,.12), 0 4px 12px rgba(0,0,0,.14)` |
| `--st-halo-recording` / `-active` / `-warning` / `-success` | `0 0 28px rgb(255 69 58/.34), 0 10px 26px rgba(0,0,0,.16)` · same with #0A84FF · `0 0 28px rgb(176 104 36/.5), …` · with #30D158 | same |

- Theme-independent tokens:
  - Radii: `--st-radius-tile:6px`, `--st-radius-selection:8px`, `--st-radius-popover:10px`, `--st-radius-card:16px`, `--st-radius-card-lg:18px`, `--st-radius-window:18px`, `--st-radius-sheet:28px`.
  - Control heights: `--st-h-row-control:28px`, `--st-h-panel-control:32px`.
  - Row heights: `--st-row-info:36px`, `--st-row-control:50px`, `--st-row-icon:64px`.
  - Type: `--st-font-body:14px`, `--st-font-caption:12px`, `--st-font-info:13px`, `--st-font-section:15px`, `--st-font-title:24px`, `--st-font-sidebar:17px`, `--st-font-family:-apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif`.
  - Z-scale: `--st-z-sheet:900`, `--st-z-popover:1000`, `--st-z-tooltip:1050`, `--st-z-toast:1100`.
  - Motion: `--st-dur-fast:120ms`, `--st-dur-base:160ms`, `--st-dur-slow:200ms`, `--st-dur-ink:250ms`, `--st-ease-out:cubic-bezier(.2,0,0,1)`.
  - Ink: `--st-ink-orange:#EF643C`, `--st-ink-red:#FF453A`, `--st-ink-amber:#FF9F0A`, `--st-ink-green:#30D158`, `--st-ink-blue:#0A84FF`, `--st-ink-purple:#BF5AF2`, `--st-ink-white:#FFFFFF`, `--st-ink-highlighter:#FFD60A`.

- [ ] **Step 1: Write the failing tests** in `tokens.test.ts`. Parse `tokens.css` with a small regex parser of `--name: value;` blocks per selector.

```ts
test('every themed token is defined for dark and light', () => {
  for (const name of THEMED) { expect(dark[name]).toBeDefined(); expect(light[name]).toBeDefined(); }
});
test('body text meets WCAG AA on window and group in both themes', () => {
  for (const t of [dark, light]) for (const bg of ['--st-window','--st-group'])
    expect(contrast(blend(t['--st-text'], t[bg]), t[bg])).toBeGreaterThanOrEqual(4.5);
});
test('secondary text meets 4.5:1 on group in both themes', ...);
test('no yellow outside the highlighter ink', () => {
  // hue 45–65° with saturation > .6 is allowed only for --st-ink-highlighter
});
```

- [ ] **Step 2:** Run `npm -w @speaktype/ui test`. Expected: FAIL, `tokens.css` not found.
- [ ] **Step 3:** Write `tokens.css` with the values above, and re-point `theme.css`.
- [ ] **Step 4:** Run `npm -w @speaktype/ui test && npm -w speaktype-webui run check && npm -w speaktype-webui test`. Expected: PASS. The existing webui tests stay green.
- [ ] **Step 5: Commit.** `feat(ui): design tokens package`

### Task 3: Motion (F2)

**Files:**
- Create: `packages/ui/src/motion/index.ts`
- Create: `packages/ui/src/motion/reduced.svelte.ts`
- Test: `packages/ui/src/motion/motion.test.ts`

**Interfaces (produces):**

```ts
export const duration: { fast: 120; base: 160; slow: 200; ink: 250 };
export const easeOut: (t: number) => number;          // matches cubic-bezier(.2,0,0,1)
export const reducedMotion: { readonly current: boolean }; // matchMedia-backed; listener attached lazily, removed when last reader's effect is torn down
export function sheet(node: Element, p?: { reduced?: boolean }): TransitionConfig;   // scale .96→1 + opacity, 200 ms; reduced: opacity, 120 ms
export function pop(node: Element, p?: { reduced?: boolean; placement?: 'below' | 'aligned' }): TransitionConfig; // opacity + 4px translateY (below) or opacity only (aligned), 120 ms
export function step(node: Element, p: { direction: 1 | -1; reduced?: boolean }): TransitionConfig; // translateX 24px·direction + opacity, 200 ms; reduced: opacity only
```

When `reduced` is omitted, each transition reads `reducedMotion.current`.

- [ ] **Step 1: Write the failing tests.**
  - `step` with `reduced:true` produces a `css(t)` containing no `transform`, with `duration` 120.
  - `sheet` has `duration` 200 and `css(0)` contains `scale(0.96)`.
  - `reducedMotion` attaches exactly one `matchMedia` listener for N readers and removes it after the last cleanup, checked with a stubbed `matchMedia` that counts listeners.
- [ ] **Step 2:** Run `npm -w @speaktype/ui test motion`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the tests again. Expected: PASS.
- [ ] **Step 5: Commit.** `feat(ui): motion primitives`

### Task 4: Controls (F2)

**Files:** `packages/ui/src/controls/`, one component per file:
- `Button.svelte`, `IconButton.svelte`, `Keycap.svelte`
- `Switch.svelte`, `Checkbox.svelte`, `Segmented.svelte`, `RadioList.svelte`
- `TextField.svelte`, `TextArea.svelte`, `SecureField.svelte`, `SearchField.svelte`, `ShortcutField.svelte`
- `Spinner.svelte`, `Progress.svelte`, `ProgressRing.svelte`, `StatusDot.svelte`

Test: `packages/ui/src/controls/controls.test.ts` (Vitest + @testing-library/svelte, jsdom).

The reference is the Components board PNG (`docs/design/reference/Components.png`). Behaviour comes from spec §7.5.

**Interfaces (props; all accept `class` and pass through `aria-*`):**
- `Button`
  - Props: `{ variant?: 'primary'|'secondary'|'plain'|'destructive'; size?: 28|32; busy?: boolean; disabled?: boolean; shortcut?: string[]; onclick? }`.
  - Destructive is a capsule with a red label, never filled.
  - `busy` keeps the width and shows a Spinner.
  - `shortcut` renders Keycaps.
- `IconButton`
  - Props: `{ label: string; shortcut?: string[]; icon: Component; size?: 28|32 }`.
  - Always has a tooltip `label` plus the shortcut.
- `Keycap`
  - Props: `{ key: string }`, 24 pt.
  - `'return'|'delete'|'tab'|'escape'` render SVG glyphs. Modifiers render as text in the order ⌃⌥⇧⌘.
  - Export `formatShortcut(keys: string[]): string[]`, which sorts modifiers and maps `Super`→⌘, `Control`→⌃, `Option|Alt`→⌥, `Shift`→⇧, `Enter`→`return`.
- `Switch`: `{ checked: boolean (bindable); disabled?; label: string }`.
- `Checkbox`: `{ checked: boolean|'mixed' (bindable); disabled?; label }`.
- `Segmented<T>`: `{ items: {value:T;label:string}[]; value: T (bindable) }`. Arrow keys move the selection.
- `RadioList<T>`: same shape, 2–4 options.
- `TextField`, `TextArea`, `SearchField`: `{ value (bindable); placeholder?; error?: string; disabled?; rows? (TextArea) }`. SearchField is a capsule with a leading magnifier and a clear button.
- `SecureField`
  - Props: `{ value (bindable); revealed: boolean (bindable); onrequestreveal?: () => Promise<boolean> }`.
  - The eye / eye-slash toggle sits at the trailing edge. It re-masks after 30 000 ms and on `window` blur.
  - Paste works while masked.
- `ShortcutField`
  - Props: `{ value: string[] (bindable); conflict?: string }`.
  - States: idle, recording (click to record), conflict (amber, with a message).
  - A trailing ✕ clears it. 32 pt capsule, one Keycap per key.
- `Progress`: `{ value: number|null }`, where null means indeterminate. `ProgressRing`: `{ value: number; size?: 16|20 }`.
- `StatusDot`: `{ tone: 'neutral'|'accent'|'success'|'warning'|'danger'|'live'; label: string }`. Visually a dot; the label is for screen readers.

- [ ] **Step 1: Write the failing tests.** At minimum:
  - `formatShortcut(['Super','Control'])` equals `['⌃','⌘']`.
  - `formatShortcut(['Shift','Super','Option','k'])` equals `['⌥','⇧','⌘','K']`.
  - `Keycap` for `return` renders an `svg` and no text node "↩".
  - A disabled `Button` ignores clicks and has `aria-disabled="true"`.
  - A busy `Button` keeps `offsetWidth` (use a mocked layout via `getBoundingClientRect` stubs) and has `aria-busy="true"`.
  - A destructive `Button` never has the class `is-default`.
  - `Segmented`: ArrowRight moves the value to the next item.
  - `SecureField` re-masks after 30 s using fake timers, and on a `blur` event on `window`.
  - `ShortcutField` in recording state captures `keydown` with ⌥⌘K and sets the value to `['Option','Super','K']`.
  - `IconButton` renders an accessible name equal to `label`.
- [ ] **Step 2:** Run `npm -w @speaktype/ui test controls`. Expected: FAIL.
- [ ] **Step 3:** Implement on bits-ui primitives where one exists (Switch, Checkbox, RadioGroup, ToggleGroup).
  - Hover is one step lighter over 120 ms, pressed one step darker, with no movement.
  - The focus ring appears only on `:focus-visible`.
  - Disabled is 40% opacity.
  - The cursor stays the arrow.
- [ ] **Step 4:** Run the tests again. Expected: PASS.
- [ ] **Step 5: Commit.** `feat(ui): controls`

### Task 5: Floating layers (F2)

**Files:** `packages/ui/src/floating/`
- `LayerRoot.svelte` (mount once per window; owns the portal target and z-scale)
- `Tooltip.svelte` plus `truncate.ts` (action)
- `Popover.svelte`
- `Select/` (Root, Trigger, Content, Item, Group, Separator)
- `Combobox/`
- `Menu/` (DropdownMenu and ContextMenu, sharing `MenuItem` with a leading check column, icon column and label column)
- `PopupMenu.svelte` (pop-up button with leading icons)

Test: `packages/ui/src/floating/floating.test.ts`.

**Interfaces:**
- `truncate(node, { mode: 'tail'|'middle'; full: string })` is a Svelte action.
  - It sets the visible text, using middle ellipsis for paths.
  - It shows a tooltip after 500 ms hover **only if** `scrollWidth > clientWidth`, or the middle-truncated text differs from `full`.
  - It returns `{ update, destroy }`. `destroy` removes every listener and its ResizeObserver.
- `Select`
  - Props: `placement?: 'aligned'|'below'`, default `'aligned'`.
  - Aligned placement opens the content so the **selected item's label sits exactly over the trigger's label**: same top within ±1 px, same left text edge within ±1 px.
  - It is clamped to the viewport with an 8 px margin. When clamped, it keeps as much alignment as fits.
  - With no selection it aligns the first item.
  - Content always renders in `LayerRoot`, above `--st-z-sheet`.
- `Combobox`
  - Always placed `'below'`, with type-to-filter.
  - Matches are bold, the highlighted option follows the arrow keys, and a no-results row reads "No matches".
- Shared menu column layout: `--menu-check-col:12px`, `--menu-icon-col:22px`, gap 10 px, row min-height 30 px, padding 5 px.
  - Every label starts at the same x, including items without an icon.
- `export function alignedOffset(triggerLabelRect: DOMRect, itemLabelOffsetTop: number, contentHeight: number, viewportHeight: number, margin = 8): number` returns the content's top. It is a pure function, so it can be unit-tested.

- [ ] **Step 1: Write the failing tests.**
  - For `alignedOffset`: the selected item in the middle of a list puts the top at `triggerTop - itemOffset`.
  - When that would go below the viewport, the result is clamped to `viewportHeight - contentHeight - 8`.
  - When it would go above, it is clamped to 8.
  - `truncate` attaches no tooltip for text that fits, attaches one for overflow (stub `scrollWidth`/`clientWidth`), and has removed all listeners after `destroy` (spy on `addEventListener`/`removeEventListener` counts).
  - `MenuItem` without an icon renders a placeholder of the icon column width.
- [ ] **Step 2:** Run `npm -w @speaktype/ui test floating`. Expected: FAIL.
- [ ] **Step 3:** Implement on bits-ui (`Select`, `Combobox`, `DropdownMenu`, `ContextMenu`, `Popover`, `Tooltip`), using their `Portal` with `to` set to the LayerRoot target. Aligned placement uses `alignedOffset` through the content's custom positioning.
- [ ] **Step 4:** Run the tests again. Expected: PASS.
- [ ] **Step 5: Commit.** `feat(ui): portal-rooted floating layers with macOS pop-up placement`

### Task 6: Containers and patterns (F2)

**Files:** `packages/ui/src/containers/`
- `Sheet.svelte`, `Alert.svelte`, `Toast.svelte` plus `toast.svelte.ts` (queue)
- `Group.svelte` plus `Row.svelte`
- `Sidebar.svelte` plus `SidebarItem.svelte`
- `EmptyState.svelte`, `Skeleton.svelte`, `ThumbnailPicker.svelte`, `SwatchRow.svelte`, `ProBadge.svelte`, `Halo.svelte`

Test: `containers.test.ts`.

**Interfaces:**
- `Sheet`
  - Props: `{ open: boolean (bindable); title: string; onclose?: () => void }`, plus `footer` and `children` snippets.
  - A 28 pt radius floating panel, centred horizontally, 72 px from the window top, over a 40% black scrim.
  - One shared inset of 28 px. Esc closes. Focus is trapped inside and restored to the opener.
  - Uses the `sheet` transition.
- `Alert`
  - Props: `{ open (bindable); title; message?; confirmLabel: string; destructive?: boolean }`. Returns a promise from the `confirm()` helper, `confirmAlert(opts): Promise<boolean>`.
  - Cancel is the default (Return). The confirm label is red when destructive.
- `toast.show({ message: string; action?: { label: string; run: () => void }; durationMs?: number = 5000 }): () => void`.
  - At most 3 visible. The newest stacks at the bottom. Toasts pause on hover.
  - The Undo pattern uses `action`.
- `Row`
  - Props: `{ kind: 'info'|'control'|'icon'; label: string; caption?: string; icon?: Component; tileColor?: string }`, plus a `trailing` snippet.
  - Heights are 36, 50 or 64 from the tokens. The label is 14 pt (13 pt for `info`) and the caption 12 pt.
  - Hairline separators sit between rows, inset 16 px.
- `Sidebar` / `SidebarItem`
  - `SidebarItem` props: `{ label; icon: Component; tileColor: string; selected: boolean; badge?: 'dot' }`.
  - Metrics: 17 pt label, 36 pt row, 24 pt tile with a 14 pt glyph and 6 pt radius, 10 pt gap, 10 pt inset, 8 pt selection radius, 2 pt between rows, 10 pt between groups.
  - Hover fades in `rgba(255,255,255,.07)` over 120 ms. Selected is an accent fill with a white label.
  - Width 256 pt.
- `ThumbnailPicker<T>`: `{ items: { value: T; label: string; preview: Snippet }[]; value (bindable) }`. The selected item gets a 3 pt blue ring and a bold label; the others have muted labels.
- `SwatchRow`: `{ colors: { value: string; name: string }[]; value (bindable) }`. 28 pt circles; the selected one has a 2 pt gap and a blue ring, with its name captioned below the row.
- `Halo`: `{ tone: 'recording'|'active'|'warning'|'success'|null }`. Applies the matching `--st-halo-*` token. It is the only component allowed to use halo tokens; a lint test greps for that.
- `EmptyState`: `{ title; message?; icon?; actions? snippet }`.
- `Skeleton`: `{ width?; height?; lines? }`. It shimmers only when not reduced-motion.

- [ ] **Step 1: Write the failing tests.**
  - `Sheet`: Esc closes it; after close, focus is on the opener; Tab cycles within the sheet.
  - `confirmAlert` resolves `false` on Return, because Cancel is the default.
  - `toast.show` ×4 leaves 3 visible.
  - The `action.run` of an Undo toast is called on click.
  - `Row` with `kind:'info'` has a computed min-height of 36px.
  - A lint test fails if `--st-halo-` appears in any file other than `Halo.svelte` and `tokens.css`.
- [ ] **Step 2:** Run `npm -w @speaktype/ui test containers`. Expected: FAIL.
- [ ] **Step 3:** Implement. `Sheet` and `Alert` are built on bits-ui `Dialog` / `AlertDialog`, portalled to LayerRoot at `--st-z-sheet`.
- [ ] **Step 4:** Run the tests again. Expected: PASS.
- [ ] **Step 5: Commit.** `feat(ui): containers and patterns`

### Task 7: Gallery and visual pipeline (F2)

**Files:**
- Create: `packages/ui/gallery/index.html`, `packages/ui/gallery/main.ts`, `packages/ui/gallery/registry.ts`, `packages/ui/gallery/pages/*.svelte`
- Create: `packages/ui/playwright.config.ts`, `packages/ui/tests/visual/gallery.spec.ts`, `packages/ui/tests/visual/behaviour.spec.ts`
- Create: `packages/ui/scripts/screens.mjs` (`npm run screens -- --out <dir>` copies the latest screenshots to `.handoff/<ws>/screens/`)
- Modify: `packages/ui/package.json` scripts `gallery`, `visual`, `visual:update`, `screens`

**Interfaces:**
- `registry.ts` exports `pages: { id: string; title: string; component: Component }[]`, one page per Components-board section, showing every state from spec §7.5.
- The URL scheme is `/#/<id>?theme=light|dark`. A query `?state=<name>` forces hover, pressed or focus via the `data-force-state` attribute, which components honour only in the gallery build.

- [ ] **Step 1: Write the specs.**
  - `gallery.spec.ts` loops over `pages × ['light','dark']`. For each it calls `expect(page).toHaveScreenshot(`${id}-${theme}.png`, { maxDiffPixelRatio: 0.001 })` at `deviceScaleFactor: 2`, with a 1280×800 viewport.
  - `behaviour.spec.ts` has these tests:
    - `select opens item-aligned and stays inside the viewport near the bottom edge`: a trigger at y = 600 in a 640 px viewport with 8 items and the 6th selected. The content's bounding box stays within [8, 632], and the selected label's top lies within 1 px of the trigger label's top, or the content is clamped.
    - `select inside a sheet renders above the sheet`: `elementFromPoint` at the content centre returns a descendant of the select content.
    - `long path middle-truncates and shows a tooltip only when truncated`: the text contains "…" between the first and last path segments. Hovering for 600 ms makes `role=tooltip` visible with the full path. For a short path, no tooltip appears after 600 ms.
    - `sheet toggled 10× in 1 s ends closed with focus restored and no inert leftovers`: after the loop there are no `[data-dialog-overlay]` elements, `document.activeElement` is the opener, and no element has `inert` or `pointer-events: none` set on `body`.
- [ ] **Step 2:** Run `npm -w @speaktype/ui run visual`. Expected: the behaviour tests pass if Tasks 4–6 are right, and the first gallery run fails with missing baselines.
- [ ] **Step 3:** Build every gallery page. Then run `npm -w @speaktype/ui run visual:update`, and compare every PNG with `docs/design/reference/Components.png` by eye (the builder model has vision). Fix any differences before committing the baselines.
- [ ] **Step 4:** Run `npm -w @speaktype/ui run visual`. Expected: PASS.
- [ ] **Step 5: Commit** the baselines under `packages/ui/tests/visual/__screenshots__/`. `feat(ui): gallery + visual regression`

---

### Task 8: Root Cargo workspace and `make verify` (F3)

**Files:**
- Create: `Cargo.toml` (root): `[workspace] members = ["apps/widget/src-tauri", "crates/*"]`, `resolver = "2"`, plus `[workspace.dependencies]` for serde, serde_json, tokio, rusqlite, thiserror, chrono.
- Move: `apps/widget/rust-toolchain.toml` → `rust-toolchain.toml` (root).
- Modify: `apps/widget/src-tauri/Cargo.toml` to use `workspace = true` for the shared dependencies.
- Modify: `Makefile`, adding a `verify:` target that runs, in order:
  - `cargo fmt --all --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `npm run -ws --if-present check`
  - `npm run -ws --if-present lint`
  - `npm run -ws --if-present test`
  - `npm -w @speaktype/ui run visual`
- Create: `.github/workflows/verify.yml` running `make verify` on `macos-14`. The workflow runs `npx -w @speaktype/ui playwright install chromium` before `make verify` (the visual suite needs it); `make verify` itself assumes it is installed.

- [ ] **Step 1:** Run `cargo test --workspace` from the root. Expected before the change: error, no workspace.
- [ ] **Step 2:** Create the workspace and move the toolchain file.
- [ ] **Step 3:** Run `cargo test --workspace && (cd apps/widget && cargo tauri build --debug --no-bundle)`. Expected: both pass. The Tauri CLI still finds `src-tauri`.
- [ ] **Step 4: Commit.** `build: root cargo workspace + make verify`

### Task 9: `speaktype-session` types and store (F3)

**Files:**
- Create: `crates/speaktype-session/Cargo.toml` (deps: rusqlite, serde, serde_json, thiserror, chrono, specta with the `derive` feature)
- Create: `crates/speaktype-session/src/lib.rs`, `src/types.rs`, `src/store.rs`, `src/schema.sql`
- Test: inline `#[cfg(test)]` in `store.rs`

**Interfaces (produces).** All types are `#[derive(Serialize, Deserialize, specta::Type, Clone, Debug, PartialEq)]` with `#[serde(rename_all = "camelCase")]`.

```rust
pub enum ItemKind { Speech, Mark, Screenshot, Clip, Note }   // serde "speech" | "mark" | …; DB event_type: transcript|mark|screenshot|clipboard|note
pub enum Target { CodingAgent, Chat, Markdown }               // serde "codingAgent" | "chat" | "markdown"
pub struct Session { pub id: i64, pub name: String, pub started_at_ms: i64, pub ended_at_ms: Option<i64>, pub template: String, pub target: Target, pub preface: Option<String>, pub bundle_path: Option<String> }
pub struct SessionSummary { pub id: i64, pub name: String, pub started_at_ms: i64, pub ended_at_ms: Option<i64>, pub mark_count: u32, pub clip_count: u32, pub snippet: Option<String> }
pub struct Item { pub id: i64, pub session_id: i64, pub kind: ItemKind, pub ord: f64, pub t_start_ms: i64, pub t_end_ms: Option<i64>, pub text: Option<String>, pub caption: Option<String>, pub image_path: Option<String>, pub source_app: Option<String>, pub payload: serde_json::Value }
pub struct NewItem { pub kind: ItemKind, pub t_start_ms: i64, pub t_end_ms: Option<i64>, pub text: Option<String>, pub image_path: Option<String>, pub source_app: Option<String>, pub payload: serde_json::Value }
pub struct ItemPatch { pub text: Option<String>, pub caption: Option<String> }
pub struct SessionDetail { pub session: Session, pub items: Vec<Item> }
pub struct Rect { pub x: f64, pub y: f64, pub width: f64, pub height: f64 } // display points

pub struct Store { /* Mutex<Connection> */ }
impl Store {
  pub fn open(path: &Path) -> Result<Self, StoreError>;      // PRAGMA user_version < 2 → drop scribe_* tables and recreate (spec §10: dev DBs only); sets WAL
  pub fn open_in_memory() -> Result<Self, StoreError>;
  pub fn create_session(&self, name: &str, template: &str, now_ms: i64) -> Result<Session, StoreError>;
  pub fn end_session(&self, id: i64, now_ms: i64) -> Result<Session, StoreError>;
  pub fn active_session(&self) -> Result<Option<Session>, StoreError>;  // ended_at IS NULL, newest
  pub fn add_item(&self, session_id: i64, item: NewItem) -> Result<Item, StoreError>; // ord = max(ord)+1
  pub fn detail(&self, session_id: i64) -> Result<SessionDetail, StoreError>;          // deleted = 0, ORDER BY ord
  pub fn update_item(&self, id: i64, patch: ItemPatch) -> Result<Item, StoreError>;
  pub fn delete_items(&self, ids: &[i64]) -> Result<(), StoreError>;    // soft delete
  pub fn restore_items(&self, ids: &[i64]) -> Result<(), StoreError>;   // Undo
  pub fn move_item(&self, id: i64, ord: f64) -> Result<Item, StoreError>;
  pub fn split_item(&self, id: i64, char_index: usize) -> Result<(Item, Item), StoreError>; // speech only; second gets ord midway to next item
  pub fn list_sessions(&self, query: Option<&str>) -> Result<Vec<SessionSummary>, StoreError>; // FTS5 over text, caption, payload.ocr
  pub fn rename_session(&self, id: i64, name: &str) -> Result<(), StoreError>;
  pub fn delete_session(&self, id: i64) -> Result<(), StoreError>;
  pub fn set_output(&self, id: i64, target: Target, template: &str, preface: Option<&str>) -> Result<Session, StoreError>;
}
```

- Tables keep the names `scribe_sessions` / `scribe_events` (spec §12: "extend, don't rename").
- Migration v2 adds `t_start_ms`, `t_end_ms`, `ord REAL`, `caption`, `deleted INTEGER DEFAULT 0`, plus `target`, `preface`, `bundle_path` on sessions. `PRAGMA user_version = 2`.

- [ ] **Step 1: Write the failing tests.**
  - `opens_v1_db_and_resets_to_v2`: build a v1 schema by hand with `user_version` 1. After `open`, `user_version` is 2 and the tables have the new columns.
  - `items_return_in_ord_order_without_deleted`
  - `move_item_between_neighbours_keeps_order`: move item 3 to `(ord1 + ord2) / 2` and the order becomes 1, 3, 2.
  - `split_item_splits_on_char_boundary_not_byte`: text "héllo wörld" split at 6 gives "héllo " and "wörld".
  - `split_non_speech_errors`
  - `delete_then_restore_round_trips`
  - `list_sessions_search_matches_clip_and_caption_text`
  - `concurrent_add_item_from_two_threads_keeps_unique_ord`: 2 threads × 100 inserts give 200 distinct ords.
- [ ] **Step 2:** Run `cargo test -p speaktype-session`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the tests again. Expected: PASS.
- [ ] **Step 5: Commit.** `feat(session): store with v2 schema`

### Task 10: Prompt assembly (F3)

**Files:**
- Create: `crates/speaktype-session/src/assemble.rs`
- Create: `crates/speaktype-session/tests/golden/{coding_agent,chat,markdown}.md`
- Create: `crates/speaktype-session/tests/assemble_golden.rs`

**Interfaces:**

```rust
pub struct RenderedPrompt { pub text: String, pub images: Vec<String> /* absolute paths in order */, pub image_count: u32, pub total_image_bytes: u64, pub warning: Option<String> }
pub fn assemble(detail: &SessionDetail, target: Target, preface: &str, bundle_dir: &Path) -> RenderedPrompt;
```

The format rules below are verbatim from spec §6.4.
- **Order and spacing:** the preface comes first, then a blank line, then the items in `ord` order. Items are separated by exactly one blank line.
- **Speech:** the text, trimmed.
- **Mark n / Screenshot n:** numbered separately per kind.
  - Coding agent: `![Mark n](<abs bundle_dir>/images/NN-mark.png)`.
  - Chat: `[Image k]`, where k counts all images.
  - Markdown: `![Mark n](images/NN-mark.png)`.
- **Clip:** a fenced block with info string `text`. The fence is one backtick longer than the longest backtick run inside the clip, minimum 3.
- **Note:** the text.
- **Empty preface:** no leading blank line.
- **Warning:** above 20 images, `warning = Some("More than 20 images. Some apps may reject this.")`.
- **Byte count:** `total_image_bytes` sums the file sizes that exist. A missing file counts 0, and `warning` then names it ("Image 3 is missing.").

- [ ] **Step 1: Write the failing tests.**
  - Golden tests: one fixture session (2 speech, 2 marks, 1 screenshot, 1 clip, in mixed `ord`), asserted byte-for-byte against each golden file.
  - `clip_containing_backticks_uses_longer_fence`: a clip with "```" gets a 4-backtick fence.
  - `deleted_items_are_excluded`
  - `twenty_one_images_sets_warning`
  - `same_input_same_output`: two calls give equal strings (determinism).
- [ ] **Step 2:** Run `cargo test -p speaktype-session assemble`. Expected: FAIL.
- [ ] **Step 3:** Implement as a pure function. No I/O except `fs::metadata` for sizes.
- [ ] **Step 4:** Run the tests again. Expected: PASS.
- [ ] **Step 5: Commit.** `feat(session): deterministic prompt assembly`

### Task 11: Bundle writer and image ops (F3)

**Files:**
- Create: `crates/speaktype-capture/Cargo.toml` (deps: image with png only, fast_image_resize), `src/lib.rs`, `src/image_ops.rs`
- Create: `crates/speaktype-session/src/bundle.rs` (depends on `speaktype-capture`)

**Interfaces:**

```rust
// speaktype-capture
pub fn downscale_png(src: &Path, dst: &Path, max_edge: u32 /* 1568 */) -> Result<(u32, u32), ImageError>; // never upscales; writes via temp + rename
// speaktype-session
pub fn slug(name: &str) -> String;  // NFKD → ASCII, lowercase, non-alnum → '-', collapse, trim '-', max 48 chars at a '-' boundary when possible, empty → "session"
pub fn bundle_dir(root: &Path, session: &Session) -> PathBuf;  // root/<yyyy-mm-dd of started_at, local tz>-<slug>; suffix -2, -3… if taken by a different session id (marker file .session-id)
pub fn write_bundle(root: &Path, detail: &SessionDetail, rendered: &RenderedPrompt) -> Result<PathBuf, BundleError>;
// writes images/NN-<kind>.png (downscaled), prompt.md, session.json {"schemaVersion":1, session, items}, each via temp file + rename (atomic); removes images of deleted items
```

The default root is `~/Documents/SpeakType/Sessions` (spec §6.5), passed in by the caller.

- [ ] **Step 1: Write the failing tests.**
  - `downscale_never_upscales`: a 800×600 input gives (800, 600).
  - `downscale_long_edge_to_1568`: a 3024×1964 input gives (1568, 1018).
  - `slug_handles_unicode_and_length`: `"Café — checkout bug!!"` gives `"cafe-checkout-bug"`; 200 × "a b " gives a slug ≤ 48 chars; `"😀"` gives `"session"`.
  - `bundle_dir_suffixes_on_collision`
  - `write_bundle_is_atomic`: no `*.tmp` files remain, and `prompt.md` equals `rendered.text`.
  - `rewrite_after_delete_removes_stale_image`
- [ ] **Step 2:** Run `cargo test -p speaktype-capture -p speaktype-session bundle`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the tests again. Expected: PASS.
- [ ] **Step 5: Commit.** `feat(session): bundle writer + image downscale`

### Task 12: Typed IPC, session commands, Scribe UI removal (F3)

**Files:**
- Modify: `apps/widget/src-tauri/Cargo.toml` (add `tauri-specta` and `specta-typescript`, pinned to exact versions that support Tauri 2; give the reason in the report; add `speaktype-session`)
- Create: `apps/widget/src-tauri/src/session_cmds.rs`
- Create: `apps/widget/src-tauri/src/ipc_export.rs`
- Modify: `apps/widget/src-tauri/src/lib.rs`
  - Build the handler with `tauri_specta::Builder`.
  - Register the new commands and remove every `*_scribe_*` command and its registration-test entries (`lib.rs:118-138`, `:604-622`).
- Modify: `apps/widget/src-tauri/src/clipboard.rs` and `capture.rs` so the clipboard and screenshot paths call `Store::add_item` on the active session, or no-op when there isn't one. Keep the existing rules (spec §12).
- Delete: `apps/widget/webui/src/settings/ScribeTab.svelte`, its import and tab in `Settings.svelte`, and the `scribe` functions in `ipc.ts`.
- Delete: `apps/widget/src-tauri/src/scribe.rs` once nothing references it. `llm.rs`, `llm_server.rs` and `templates.rs` stay compiled (spec §10).
- Generate: `apps/widget/webui/src/lib/bindings.ts`

**Interfaces (produces).** Every command is `#[tauri::command] #[specta::specta]` and returns `Result<T, IpcError>`, where `IpcError { kind: String, message: String }` is serde camelCase (the normalised `{kind, message}`, spec §6.1).
- Commands, as in spec §7.4:
  - `session_start(template: Option<String>) -> Session`
  - `session_stop() -> Session`
  - `session_active() -> Option<Session>`
  - `session_set_mute(muted: bool)`: stores state only, no narration yet
  - `session_list(query: Option<String>) -> Vec<SessionSummary>`
  - `session_get(id: i64) -> SessionDetail`
  - `session_rename(id, name)`, `session_delete(id)`
  - `item_update(id, patch: ItemPatch) -> Item`
  - `item_delete(ids: Vec<i64>)`, `item_restore(ids: Vec<i64>)`
  - `item_move(id, ord: f64) -> Item`, `item_split(id, char_index: u32) -> (Item, Item)`
  - `prompt_render(session_id, target: Target) -> RenderedPrompt`
  - `prompt_copy(session_id, target)`: writes `rendered.text` to the clipboard via the existing `arboard` path
  - `bundle_reveal(session_id)`
  - `overlay_pen(on: bool)` and `overlay_mark_complete(display_id: u32, bounds: Rect)`: stubs returning `Ok(())`, logged
- Typed events (`tauri_specta::Event`):
  - `SessionState { session: Option<Session>, phase: "idle"|"active"|"stopping", muted: bool }`
  - `SessionItemAdded { item: Item }`
  - `SessionItemUpdated { item: Item }`
  - `SessionItemsDeleted { ids: Vec<i64> }`
- Every mutating command emits the matching event **after** the DB write commits.
- The existing non-scribe commands and events are exported through specta too, so `bindings.ts` covers the whole app. `ipc.ts` becomes `export * from './bindings'` plus the small helpers that are still used.

- [ ] **Step 1: Write the failing tests.**
  - `every_frontend_command_is_registered`: updated, with the scribe names removed and the session names added.
  - New `export_bindings` test: it writes `bindings.ts` to a temp path and asserts it equals the committed file.
  - `session_stop_emits_state_after_commit`: with a mock emitter, the event is recorded after `end_session`'s row is visible from a second connection.
- [ ] **Step 2:** Run `cargo test -p speaktype`. Expected: FAIL.
- [ ] **Step 3:** Implement. Regenerate the bindings with `cargo test -p speaktype export_bindings -- --ignored --nocapture` (the writer variant), then commit them.
- [ ] **Step 4:** Run `make verify`. Expected: PASS. `npm run check` passes with `ScribeTab` gone and every caller of the old IPC moved to `bindings.ts`.
- [ ] **Step 5: Commit.** `feat(ipc): typed tauri-specta bindings + session commands; remove Scribe UI`

---

### Task 13: `speaktype-stt` provider model and secrets (F4)

**Files:**
- Create: `crates/speaktype-stt/Cargo.toml` (reqwest with json, multipart and rustls; tokio; tokio-util; async-trait; thiserror; serde; url; bytes; keyring; specta)
- Create: `crates/speaktype-stt/src/lib.rs`, `config.rs`, `secrets.rs`, `error.rs`

**Interfaces (produces):**

```rust
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ProviderConfig { Local { model: String }, Server { base_url: String }, Cloud { service: CloudService, base_url: String, model: String, fallback_to_local: bool } }
pub enum CloudService { OpenAi, Groq, Deepgram, Custom }
impl CloudService { pub fn default_base_url(&self) -> Option<&'static str>; } // OpenAi "https://api.openai.com/v1", Groq "https://api.groq.com/openai/v1", Deepgram "https://api.deepgram.com/v1", Custom None
pub trait SecretStore: Send + Sync { fn get(&self, account: &str) -> Result<Option<String>, SecretError>; fn set(&self, account: &str, value: &str) -> Result<(), SecretError>; fn delete(&self, account: &str) -> Result<(), SecretError>; }
pub struct KeychainStore; // keyring service "app.speaktype", account "stt.<service>" / "llm.api_key"
pub struct MemoryStore;   // tests
pub enum SttError { Unauthorized, RateLimited { retry_after_ms: Option<u64> }, Unreachable, Timeout, Server { status: u16 }, BadResponse(String), Cancelled, NotConfigured }
impl SttError { pub fn user_message(&self, service_name: &str) -> String; pub fn is_transient(&self) -> bool; }
```

`user_message` copy:

| Error | Message |
|---|---|
| Unauthorized | "<service> didn't accept the API key." |
| RateLimited | "<service> is busy. Trying again…" |
| Unreachable | "Couldn't reach <service>." |
| Timeout | "<service> took too long to answer." |
| Server | "<service> had a problem (error <status>)." |
| BadResponse | "<service> sent an answer SpeakType couldn't read." |
| NotConfigured | "Choose a speech service in Settings → Providers." |

- [ ] **Step 1: Write the failing tests.**
  - Each config variant round-trips through serde JSON.
  - `default_base_url` holds the values above.
  - `MemoryStore` round-trips set, get and delete.
  - `is_transient` is true only for RateLimited, Unreachable, Timeout and Server with status ≥ 500.
  - `user_message` holds the exact strings above for the service "Groq".
- [ ] **Step 2:** Run `cargo test -p speaktype-stt`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the tests again. Expected: PASS.
- [ ] **Step 5: Commit.** `feat(stt): provider config, errors, keychain secrets`

### Task 14: Providers, retry and timeouts (F4)

**Files:**
- Create: `crates/speaktype-stt/src/provider.rs`, `local.rs`, `openai.rs`, `deepgram.rs`, `retry.rs`
- Test: `crates/speaktype-stt/tests/providers.rs` (wiremock)

**Interfaces:**

```rust
pub struct TranscribeRequest { pub wav: bytes::Bytes /* 16 kHz mono s16 WAV */, pub duration_ms: u32, pub language: Option<String>, pub prompt: Option<String> }
pub struct Transcript { pub text: String, pub segments: Vec<Segment>, pub latency_ms: u32 }
pub struct Segment { pub start_ms: u32, pub end_ms: u32, pub text: String, pub no_speech_prob: Option<f32>, pub avg_logprob: Option<f32>, pub compression_ratio: Option<f32> }
pub struct RemoteModel { pub id: String }
#[async_trait] pub trait SttProvider: Send + Sync {
  async fn transcribe(&self, req: TranscribeRequest, cancel: CancellationToken) -> Result<Transcript, SttError>;
  async fn list_models(&self, cancel: CancellationToken) -> Result<Vec<RemoteModel>, SttError>;
}
pub fn provider_for(cfg: &ProviderConfig, secrets: &dyn SecretStore, http: reqwest::Client) -> Result<Box<dyn SttProvider>, SttError>;
pub struct RetryPolicy { pub max_attempts: u32 /* 3 */, pub base_ms: u64 /* 500 */, pub jitter: f64 /* 0.2 */ }
pub fn timeout_for(duration_ms: u32) -> Duration; // max(10 s, 3 × duration), capped at 120 s
```

**Behaviour:**
- **Local and Server.**
  - Move the request code from `transcribe.rs:205-275` (`transcribe_once`) without changing behaviour.
  - Keep the existing quality-field parsing, so `quality.rs` keeps working.
- **OpenAI-compatible (OpenAi, Groq, Custom).**
  - Request: `POST {base}/audio/transcriptions`, multipart `file` (wav), `model` = **the provider's model**, `response_format=verbose_json`, `language` when set.
  - Header `Authorization: Bearer <key>`.
  - `list_models`: `GET {base}/models`, filtered to ids containing `whisper` or `transcribe`.
  - Curated fallback when listing fails with 404: OpenAi `[gpt-4o-transcribe, gpt-4o-mini-transcribe, whisper-1]`, Groq `[whisper-large-v3-turbo, whisper-large-v3]`.
- **Deepgram.**
  - Request: `POST {base}/listen?model=<m>&language=<l>&smart_format=true`, body `audio/wav`.
  - Header `Authorization: Token <key>`.
  - `list_models` returns the curated `[nova-3, nova-2]`.
- **Retries and timeouts.**
  - `RetryPolicy` retries `is_transient` errors only. The backoff is `base_ms × 2^(attempt-1) × (1 ± jitter)`. On 429 it uses `Retry-After` when present.
  - Each attempt is bounded by `timeout_for`.
  - `cancel` aborts at once with `Cancelled`.

- [ ] **Step 1: Write the failing tests** (wiremock):
  - `openai_sends_bearer_and_provider_model`: asserts the header `Authorization: Bearer sk-test` and the multipart field `model=whisper-large-v3-turbo`.
  - `deepgram_sends_token_header`
  - `retries_503_then_succeeds`: 2× 503, then 200, gives Ok with 3 requests seen.
  - `never_retries_401`: exactly 1 request, Err(Unauthorized).
  - `honours_retry_after_on_429`: `Retry-After: 1`. The second request arrives ≥ 1000 ms later (measured with `tokio::time::pause`).
  - `times_out_and_cancels`: the server delays 30 s, `timeout_for(1000)` is 10 s, and the paused-time test gives Err(Timeout). A separate test cancels the token after 100 ms and gets Err(Cancelled).
  - `list_models_filters_to_speech`
  - `local_parses_quality_fields`: uses the existing fixture JSON from `transcribe.rs` tests.
- [ ] **Step 2:** Run `cargo test -p speaktype-stt --test providers`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the tests again. Expected: PASS.
- [ ] **Step 5: Commit.** `feat(stt): local, OpenAI-compatible and Deepgram providers with retry/timeout/cancel`

### Task 15: Wire providers into the app (F4)

**Files:**
- Modify: `apps/widget/src-tauri/src/transcribe.rs`: it becomes a thin adapter over `speaktype_stt::provider_for`. Delete `is_openai_api` (`:38-48`).
- Modify: `apps/widget/src-tauri/src/settings.rs`:
  - Add `stt_provider: ProviderConfig`, filled on load when it's missing. A default local `api_url` becomes `Local { model: settings.model }`; any other `api_url` becomes `Server { base_url: api_url }`.
  - Move a non-empty `scribe_llm_api_key` into `KeychainStore` (account `llm.api_key`) and blank it in the JSON.
- Modify: `apps/widget/src-tauri/src/commands.rs`. Add these specta commands:
  - `stt_set_provider(cfg)`
  - `stt_set_api_key(service, key)`: Keychain only, never returned
  - `stt_has_api_key(service) -> bool`
  - `stt_reveal_api_key(service) -> String`: only through Keychain, which prompts per macOS
  - `stt_list_models() -> Vec<RemoteModel>`
  - `stt_test() -> { latencyMs, text }`: sends the bundled 3 s clip `assets/test-clip-3s.wav`

  Fallback: when the Cloud provider fails with `Unreachable` or `Timeout` and `fallback_to_local` is set, re-run the request on Local and emit `sidecar:error`-shaped `{kind:"providerFallback", message:"Couldn't reach <service>. Used On this Mac."}`.
- Regenerate `bindings.ts`.

- [ ] **Step 1: Write the failing tests.**
  - `settings_migrates_default_url_to_local`
  - `settings_migrates_custom_url_to_server`
  - `llm_key_moves_to_keychain_and_blanks_json`, using `MemoryStore` injected through a `SecretStore` parameter.
  - `cloud_unreachable_falls_back_to_local_when_enabled`, using two wiremock servers.
- [ ] **Step 2:** Run `cargo test -p speaktype settings transcribe`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run `make verify`. Expected: PASS.
- [ ] **Step 5: Commit.** `feat(stt): explicit provider setting, keychain keys, cloud fallback`

---

### Task 16: Data layer (F5, after Task 12)

**Files:**
- Create: `apps/widget/webui/src/lib/data/bus.ts`, `useEvent.svelte.ts`, `liveQuery.svelte.ts`, `index.ts`
- Test: `useEvent.test.ts`, `liveQuery.test.ts`
- Create: `apps/widget/webui/eslint.config.js` (flat config with `eslint-plugin-svelte`)
  - `no-restricted-imports` bans `@tauri-apps/api/event` and the `events` export of `$lib/bindings` everywhere except `src/lib/data/**`.
  - A rule bans `setInterval` outside `src/lib/data/**`, with the message "use liveQuery({ pollMs })".
- Modify: `apps/widget/webui/package.json`: add `"lint": "eslint src"`.

**Interfaces (produces):**

```ts
export interface EventBus { listen<T>(name: string, cb: (payload: T) => void): Promise<() => void>; }
export function setBus(bus: EventBus): void;          // tests inject a counting fake; default wraps @tauri-apps/api/event
export function useEvent<T>(name: string, cb: (payload: T) => void): void; // call during component init; unlisten via $effect teardown; safe if teardown happens before listen() resolves
export function liveQuery<T>(key: string, fetcher: () => Promise<T>, opts: { invalidateOn: string[]; pollMs?: number; refetchOnFocus?: boolean /* default true */ }):
  { readonly data: T | undefined; readonly error: unknown; readonly loading: boolean; refetch(): Promise<void> };
// shared cache per key, ref-counted across components/windows-in-process; one in-flight fetch per key; responses applied only if their sequence number is the latest; when refcount hits 0: unlisten everything, stop polling, evict after 30 s
```

- [ ] **Step 1: Write the failing tests.**
  - `useEvent.test.ts › releases every listener after 200 mount/unmount cycles`: a fake bus counts live listeners. After the loop, the count is 0.
  - `useEvent.test.ts › unmount before listen resolves still unlistens`: the fake bus resolves `listen` after the component is destroyed. The returned unlisten is still called.
  - `liveQuery.test.ts › refetches when an invalidating event arrives`: the fetch count goes from 1 to 2 and `data` updates.
  - `liveQuery.test.ts › a slow earlier response never overwrites a newer one`: the first fetch resolves after the second, and `data` equals the second result.
  - `liveQuery.test.ts › two components with the same key share one fetch`
  - `liveQuery.test.ts › stops polling and unlistens when the last user unmounts`: uses fake timers. No fetch after unmount, and the bus listener count is 0.
- [ ] **Step 2:** Run `npm -w speaktype-webui test data`. Expected: FAIL.
- [ ] **Step 3:** Implement. Migrate the one existing `listen()` call site (`lib/ipc.ts:293`) and every `Widget.svelte` subscription to `useEvent`.
- [ ] **Step 4:** Run `npm -w speaktype-webui run lint && npm -w speaktype-webui test && make verify`. Expected: PASS, with 0 lint errors.
- [ ] **Step 5: Commit.** `feat(webui): event-driven data layer with leak-proof subscriptions`

---

## Phase 0 exit (Claude)

- [ ] Every workstream has been merged to `main` through the review gate in program plan §5.
- [ ] `make verify` is green on `main`.
- [ ] Gallery screenshots in both themes have been reviewed against `docs/design/reference/Components.png` and approved.
- [ ] I've written the Phase 1 plan (`2026-11-xx-phase1-core-loop.md`) against the merged interfaces, and asked you before pushing `main` to `origin`.
