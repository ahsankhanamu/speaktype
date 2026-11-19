import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests/visual',
  snapshotPathTemplate: '{testDir}/__screenshots__/{arg}{ext}',
  outputDir: './test-results',
  reporter: 'list',
  expect: { toHaveScreenshot: { scale: 'device' } },
  use: {
    baseURL: 'http://127.0.0.1:5198',
    viewport: { width: 1040, height: 900 },
    deviceScaleFactor: 2,
  },
  webServer: {
    command: 'npx vite --config tests/visual/vite.harness.config.ts',
    url: 'http://127.0.0.1:5198/tests/visual/harness.html',
    reuseExistingServer: true,
  },
});
