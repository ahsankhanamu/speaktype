import { defineConfig } from 'vite';
import { svelte, vitePreprocess } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  root: 'gallery',
  plugins: [svelte({ preprocess: vitePreprocess() })],
  server: { port: 5199, strictPort: true },
});
