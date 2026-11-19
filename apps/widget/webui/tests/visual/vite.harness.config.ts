import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');

export default defineConfig({
  root,
  plugins: [svelte()],
  resolve: { alias: { $lib: resolve(root, 'src/lib') } },
  // The host is pinned so the readiness check and the tests reach the same
  // address: on this Mac `localhost` resolves to ::1, which the server may not
  // bind, and other worktrees share port 5198.
  server: { host: '127.0.0.1', port: 5198, strictPort: true },
});
