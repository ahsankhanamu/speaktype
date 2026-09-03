# SpeakType Frontend Migration to Vite + Svelte

**Status:** Plan (not started)
**Goal:** Migrate the frontend from plain HTML + vanilla JS to **Vite + Svelte (no SvelteKit)**, incrementally, per-page, without ever breaking the shipped app.

---

## Current state (ground truth)

- **No build step.** Tauri `frontendDist` points directly at `apps/widget/web` (raw HTML). `withGlobalTauri: true`, **no CSP**.
- **4 pages**, each loading a subset of shared global scripts:
  - `index.html` (825 ln) — the floating widget; uses `window.__TAURI__` (window/event/dpi) directly.
  - `settings.html` (420 ln) — the big one.
  - `onboarding.html` (356 ln).
  - `about.html` (108 ln).
- **Scripts:** `theme.js`, `format.js`, `waveform.js`, `ipc.js`, `model-grid.js`, `mic-test.js`, `st-select.js`, `settings.js` (2016 ln), `onboarding.js`. (~4.8k JS lines total.)
- **Styles:** `theme.css` (shared tokens), `settings.css` (1715 ln), `model-grid.css`, `widget.css`. (~3k CSS lines total.)
- All IPC goes through `ipc.js` → `window.__TAURI__.core.invoke`.
- **Rust is the contract owner.** 25 Rust tests are the safety net; command names/signatures must never change.

---

## Guiding principles

1. **Rust is the contract owner.** Every frontend command calls the *same* Rust command name/signature. `cargo test` (25) must pass after every phase.
2. **A page is either fully old or fully new.** Tauri serves whichever is active. Never a half-migrated page.
3. **CSS variables first.** All Svelte components consume existing `theme.css` tokens so dark/light works for free. No hardcoded colors.
4. **Rollback path.** Keep the old `web/` tree plus a toggle to serve it if a Svelte page regresses.
5. **Behavior preservation over cleanup.** Keep existing quirks faithfully; only refactor what is trivially safe.

---

## Setup / framework choices (confirmed)

- **Vite + Svelte** (no SvelteKit). Manual per-page root rendering via multiple Vite HTML entries.
- **TypeScript** + **vitest** for pure-logic tests.
- **Per-page migration**, keeping old UI until each new page is swapped in.
- **Shared widgets are converted to Svelte components** during migration.

---

## Phases

### Phase 0 — Foundations (no behavior change)
1. Scaffold `apps/widget/webui/` with Vite + Svelte (`vite`, `@sveltejs/vite-plugin-svelte`, `svelte`, `typescript`, `vitest`). Scripts: `vite dev`, `vite build` → `webui/dist`.
2. **Typed IPC bridge** `webui/src/lib/ipc.ts`: one typed function per Rust command, mirroring `ipc.js` (e.g. `deleteDebugSession`, `getSettings`). Old `web/` keeps using `ipc.js`.
3. Move pure logic first as TS modules: `format.ts`, `theme.ts` (from `theme.js`), `waveform.ts` (from `waveform.js`).
4. Convert shared JS widgets to components in `webui/` (built but **not yet referenced**): `StSelect.svelte` (from `st-select.js`), `ThemeTabs.svelte`, `ModelGrid.svelte`, `MicTest.svelte`.
5. Add `vite.config.ts` with `base: './'`, `build.outDir: 'dist'`, Svelte a11y warnings enabled.

**Exit:** `vite build` succeeds; `cargo test` (25) green; Rust untouched.

### Phase 1 — Migrate About (108 ln)
- `webui/src/about/main.ts` + `About.svelte`, using `ipc.ts` (opens URL via typed bridge) + `theme`.
- **Per-page served swap:** in `tauri.conf.json`, point only this page's window URL at the built page (keep others on `web/`). Use a Vite multi-entry build so each window loads its own built page.

**Exit:** About served from build, identical behavior; other pages still from `web/`.

### Phase 2 — Migrate the widget `index.html` (825 ln)
- `Widget.svelte` + `Waveform.svelte` component.
- Replace direct `window.__TAURI__` window/event/dpi calls with `@tauri-apps/api/window`, `@tauri-apps/api/event`, `@tauri-apps/api/dpi` (or a thin typed wrapper mirroring `ipc.ts`).
- Use `format.ts`/`theme.ts`.

**Exit:** Widget from build, full parity (float, reposition, multi-monitor clamp, capture states).

### Phase 3 — Migrate `onboarding.html` (356 ln)
- `Onboarding.svelte` using `ipc.ts`, `ModelGrid.svelte`, `theme.ts`.

**Exit:** Onboarding from build, full parity.

### Phase 4 — Migrate Settings (the big one: `settings.js` 2016 ln + `model-grid.js` 771 + `mic-test.js` 470)
One component per tab, sharing a Svelte store that replaces `loadedSnapshot` / `checkDirty`:
- `settings/main.ts` + `Settings.svelte` (sidebar + tab shell)
- `HotkeyPanel.svelte`
- `ServerPanel.svelte`
- `GeneralPanel.svelte` (Appearance, Permissions, Microphone)
- `ModelsPanel.svelte` (wraps `ModelGrid.svelte`)
- `HistoryPanel.svelte`
- `DebugPanel.svelte`
- `MicTest.svelte` (canvas waveform)
- `StSelect.svelte`
- `InfoPopover.svelte` (from the reusable `info-icon` + `aria-controls` pattern already added)
- shared `stores.ts`

**Preserve deliberately in the rewrite:**
- Dirty-check snapshots + Save enable state.
- `populateInputDevices` preserving an unsaved selection across background refreshes.
- Mic-test watchdog + per-device capture.
- Debug session render + per-session delete button.
- Crossfade waveform playback.

**Exit:** Settings entirely from build; each tab verified against the current UI before switching `frontendDist`.

### Phase 5 — Swap the entry + finish hardening
1. Point all 4 window URLs (`main`, `settings`, `onboarding`, `about`) at the built pages; set `frontendDist` to `webui/dist`.
2. Add strict CSP (`default-src 'self'`) and turn **off** `withGlobalTauri`; ensure `ipc.ts` / `@tauri-apps/api` cover every call.
3. Delete legacy `web/` **only after** a release DMG verifies.

**Exit:** Whole app runs from Svelte build output; `cargo build --release` + `cargo test` green.

### Phase 6 — Regression net / final de-risking
1. **vitest** tests for `format.ts`, `ipc.ts` argument-building, and any pure logic.
2. **Rust integration test** asserting the IPC command surface the UI relies on (the single biggest "don't break" protection).
3. Keep a legacy toggle (`VITE_USE_LEGACY`) until the next major release.

---

## Risk register & mitigations

| Risk | Mitigation |
|---|---|
| IPC contract drift breaks the UI | Rust integration test on command surface; never rename commands |
| CSS/theme regressions | All components consume `theme.css` tokens; per-phase parity check |
| 2000-line settings rewrite is error-prone | Per-tab migration, each tab fully verified before advancing |
| Canvas/widget animation differs | `Waveform.svelte`/`MicTest.svelte` re-implemented & validated against current |
| Build toolchain breaks Tauri startup | Duplicate `webui/`, per-page served swap, rollback toggle, `withGlobalTauri` kept until Phase 5 |

---

## Commit sequence

1. `chore(webui): scaffold Vite+Svelte, add typed ipc + shared components`
2. `feat(webui): migrate About`
3. `feat(webui): migrate widget`
4. `feat(webui): migrate onboarding`
5. `feat(webui): migrate settings panel-by-panel` (split across several commits)
6. `feat(webui): swap entry to build output, harden CSP/global, remove legacy`

---

## Notes
- **Phase 4 (Settings) is the bulk** (~60% of effort). Phases 0–3 establish the patterns (typed IPC, `StSelect`, `ModelGrid`, `MicTest`, stores) that de-risk it.
- Before starting Phase 0, commit any currently-pending UI/debug changes so the migration begins from a clean tree (ideally on a dedicated `svelte-migration` branch).