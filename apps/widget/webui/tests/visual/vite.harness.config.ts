import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');

export default defineConfig({
  root,
  plugins: [svelte()],
  resolve: { alias: { $lib: resolve(root, 'src/lib') } },
  server: { port: 5198, strictPort: true },
});
