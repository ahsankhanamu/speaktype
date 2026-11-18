import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests/visual',
  snapshotPathTemplate: '{testDir}/__screenshots__/{arg}{ext}',
  outputDir: './test-results',
  reporter: 'list',
  expect: { toHaveScreenshot: { scale: 'device' } },
  use: {
    baseURL: 'http://localhost:5199',
    viewport: { width: 1280, height: 800 },
    deviceScaleFactor: 2,
  },
  webServer: {
    command: 'npm run gallery',
    url: 'http://localhost:5199',
    reuseExistingServer: true,
  },
});
