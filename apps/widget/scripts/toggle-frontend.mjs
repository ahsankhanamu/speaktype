#!/usr/bin/env node
// Rollback toggle between the migrated Svelte build and the legacy web tree.
//
// Usage:
//   node scripts/toggle-frontend.mjs webui    # serve webui/dist (Svelte, default)
//   node scripts/toggle-frontend.mjs legacy    # serve web/ (old plain HTML+JS)
//
// The legacy tree uses `withGlobalTauri: true` and has no CSP, so this helper
// restores that config too. Run `node scripts/toggle-frontend.mjs webui` to
// switch back before shipping.
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const configPath = join(root, 'src-tauri', 'tauri.conf.json');
const config = JSON.parse(readFileSync(configPath, 'utf8'));

const arg = (process.argv[2] || 'webui').toLowerCase();
if (arg !== 'webui' && arg !== 'legacy') {
  console.error(`Unknown target "${arg}". Use "webui" or "legacy".`);
  process.exit(1);
}

const isLegacy = arg === 'legacy';

// Paths are relative to src-tauri/.
const frontendDist = isLegacy ? '../../../apps/widget/web' : '../../../apps/widget/webui/dist';
const withGlobalTauri = isLegacy;
const security = isLegacy
  ? undefined
  : {
      csp: "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; connect-src 'self' http://127.0.0.1:*; script-src 'self'",
    };

config.build = {
  devUrl: 'http://localhost:5173',
  frontendDist,
  beforeDevCommand: 'npm run dev --prefix ../../apps/widget/webui',
  beforeBuildCommand: 'npm run build --prefix ../../apps/widget/webui',
};
config.app = config.app || {};
config.app.withGlobalTauri = withGlobalTauri;
if (security) config.app.security = security;
else delete config.app.security;

writeFileSync(configPath, JSON.stringify(config, null, 2) + '\n');
console.log(
  isLegacy
    ? 'Serving LEGACY web/ (rollback). withGlobalTauri=true, no CSP.'
    : 'Serving SVELTE webui/dist. withGlobalTauri=false, CSP enabled.',
);