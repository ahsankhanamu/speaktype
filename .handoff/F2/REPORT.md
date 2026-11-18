# F2 — @speaktype/ui (Claude Sonnet 5.5) · reviewed by Claude Opus

## Commits
411f6cd tokens · d157b29 motion · 7121db4 controls · b50dd81 floating layers · bb15530 containers · 72a0c0c gallery + visual · f717e42 review round 1

## Final checks (re-run by reviewer)
- `npm -w @speaktype/ui test` — 56 passed · `run check` — 0 errors · `run visual` — 33 passed (22 screenshots + 11 behaviour), stable over 2 runs
- `npm -w speaktype-webui run check` — 0 errors · `test` — 31 passed · `run build` — ok

## Size
Own code 16.8 KB JS gzip, 3.9 KB CSS gzip (budget 60 KB). With bits-ui + lucide inlined: 91.6 KB. Largest component: Button.svelte, 126 lines.

## New dependencies
@testing-library/svelte (component tests), @playwright/test (visual/behaviour). bits-ui, @lucide/svelte, svelte as peers.

## Deviations (accepted in review)
- Focus ring follows the spec (hugging 1.5 pt), not the old gap ring on boards; primary adds a 1 px white inset so it shows on blue.
- Hover/pressed/focus forced in screenshots via CDP `CSS.forcePseudoState` — no gallery-only code in components.
- Extra themed tokens beyond the plan list (popover, sheet elevation, fills, link, danger-text, keycap, scrim…); system-light block asserted equal to explicit light block.
- `SidebarGroup` added (board has groups with 10 pt gaps); `Combobox` exposes bindable `open`/`query`.
- Aligned pop-ups swallow the opening click's `pointerup` unless the pointer moved > 4 px (bits-ui selects on pointerup).

## Review round 1 (f717e42)
Invisible focus ring on primary/plain (`none` inside a box-shadow list) · pop-up narrower than its trigger · heavy dark outline on light sheets (new `--st-elevation-sheet`) · Combobox "No results" caption placement · truncate double timer · .gitignore node_modules/test-results · stale webui lockfile removed.

## Known gaps / follow-ups
- Combobox open state highlights the selected item until typing (board highlights first match).
- Real-device check of the pop-up first-`pointerup` guard with a physical mouse when a screen first uses it (D3).
- Visual baselines: after thin visual changes, delete `packages/ui/tests/visual/__screenshots__/*.png` before `visual:update` (0.1% threshold keeps stale baselines).
- CI must run `npx -w @speaktype/ui playwright install chromium` before `make verify` (F3, Task 8).
