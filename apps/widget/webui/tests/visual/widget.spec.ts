import { expect, test, type Page } from '@playwright/test';

const STATES = [
  'idle',
  'recording',
  'transcribing',
  'preparing',
  'no-speech',
  'pasted',
  'copied',
  'mic-unavailable',
  'mic-fallback',
  'server-error',
  'long-audio',
];

// The page clock is frozen, so a hold time (pasted: 0.5 s, no speech: 2 s) cannot run out mid-screenshot.
async function open(page: Page, state: string, theme: string, extra = '') {
  await page.clock.install({ time: 0 });
  await page.clock.pauseAt(1000);
  await page.goto(`/tests/visual/scenes/widget.html?state=${state}&theme=${theme}${extra}`);
  await page.waitForFunction(() => window.__mock);
  await page.clock.runFor(200);
}

for (const theme of ['dark', 'light']) {
  test.describe(theme, () => {
    test.use({ viewport: { width: 560, height: 640 } });

    for (const state of STATES) {
      test(state, async ({ page }) => {
        await open(page, state, theme);
        await expect(page.locator('.widget')).toHaveScreenshot(`widget-${state}-${theme}.png`);
      });
    }

    test('recording with speech', async ({ page }) => {
      await open(page, 'recording', theme);
      const speech = [0.01, 0.03, 0.06, 0.02, 0.05, 0.07, 0.03, 0.01, 0.04, 0.06, 0.02, 0.05];
      for (let step = 0; step < 117; step++) {
        const level = speech[(step * 5) % speech.length];
        await page.evaluate((value) => window.__mock.emit('sidecar:audio_level', { level: value, peak: value }), level);
        await page.clock.runFor(60);
      }
      await expect(page.getByText('0:07')).toBeVisible();
      await expect(page.locator('.widget')).toHaveScreenshot(`widget-recording-speech-${theme}.png`, { maxDiffPixelRatio: 0.03 });
    });

    test("You're all set card", async ({ page }) => {
      await open(page, 'all-set', theme);
      await expect(page.getByText("You're all set")).toBeVisible();
      await expect(page.locator('.widget')).toHaveScreenshot(`widget-all-set-${theme}.png`);
    });

    test('finish setup dot and server dot', async ({ page }) => {
      await open(page, 'idle', theme, '&setup=unfinished');
      await expect(page.getByRole('img', { name: 'Finish setup' })).toBeVisible();
      await expect(page.locator('.widget')).toHaveScreenshot(`widget-finish-setup-${theme}.png`);
      await page.evaluate(() => window.__mock.emit('server:starting', {}));
      await expect(page.getByRole('img', { name: 'Server starting…' })).toBeVisible();
      await expect(page.locator('.widget')).toHaveScreenshot(`widget-server-starting-${theme}.png`);
    });

    for (const size of ['small', 'large']) {
      test(`${size} widget`, async ({ page }) => {
        await open(page, 'copied', theme, `&size=${size}`);
        await expect(page.locator('.widget')).toHaveScreenshot(`widget-${size}-copied-${theme}.png`);
      });
    }

    test('idle hover shows the shortcut', async ({ page }) => {
      await open(page, 'idle', theme);
      await page.getByRole('button', { name: 'Dictate' }).hover();
      await page.clock.runFor(700);
      await expect(page.getByRole('tooltip')).toBeVisible();
      await expect(page).toHaveScreenshot(`widget-hover-${theme}.png`);
    });

    test('right-click menu', async ({ page }) => {
      await open(page, 'idle', theme);
      await page.getByRole('button', { name: 'Dictate' }).click({ button: 'right' });
      await expect(page.getByRole('menuitem', { name: 'Hide widget' })).toBeVisible();
      await expect(page).toHaveScreenshot(`widget-menu-${theme}.png`);
    });

    test('microphone picker from the menu', async ({ page }) => {
      await open(page, 'idle', theme);
      await page.getByRole('button', { name: 'Dictate' }).click({ button: 'right' });
      await page.getByRole('menuitem', { name: 'Microphone' }).click();
      await expect(page.getByText('AirPods Pro')).toBeVisible();
      await expect(page).toHaveScreenshot(`widget-mic-${theme}.png`);
    });

    test('speech model from the menu', async ({ page }) => {
      await open(page, 'idle', theme);
      await page.getByRole('button', { name: 'Dictate' }).click({ button: 'right' });
      await page.getByRole('menuitem', { name: 'Speech model' }).click();
      await expect(page.getByRole('combobox', { name: 'Speech model' })).toBeVisible();
      await expect(page).toHaveScreenshot(`widget-model-${theme}.png`);
    });
  });
}

for (const theme of ['dark', 'light']) {
  test(`every state on one sheet, ${theme}`, async ({ page }) => {
    await page.setViewportSize({ width: 1140, height: 560 });
    await page.clock.install({ time: 0 });
    await page.clock.pauseAt(1000);
    await page.goto(`/tests/visual/scenes/widget-sheet.html?theme=${theme}`);
    await page.waitForLoadState('load');
    await page.clock.runFor(400);
    await expect(page).toHaveScreenshot(`widget-sheet-${theme}.png`);
  });
}

test('Choose mic opens the picker in place', async ({ page }) => {
  await page.setViewportSize({ width: 560, height: 640 });
  await open(page, 'mic-unavailable', 'dark');
  await page.getByRole('button', { name: 'Choose mic' }).click();
  await expect(page.getByText('System default')).toBeVisible();
  const calls = await page.evaluate(() => window.__mock.calls.map((call) => call.cmd));
  expect(calls).toContain('mic_levels_start');
});
