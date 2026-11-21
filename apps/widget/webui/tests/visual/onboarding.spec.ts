import { expect, test, type Page } from '@playwright/test';

const open = (page: Page, fixture: string, theme: string) =>
  page.goto(`/tests/visual/scenes/onboarding.html?fixture=${fixture}&theme=${theme}`);

const steps = [
  { name: 'welcome', fixture: 'welcome', ready: 'Welcome to SpeakType.' },
  { name: 'microphone', fixture: 'mic', ready: 'Microphone allowed. Say something.' },
  { name: 'microphone-waiting', fixture: 'mic-denied', ready: 'Waiting for microphone access…' },
  { name: 'accessibility', fixture: 'accessibility', ready: 'Waiting for you to switch it on.' },
  { name: 'speech-model', fixture: 'model', ready: 'Fast ready · Balanced 64%' },
  { name: 'try-it', fixture: 'try', ready: 'Try it once.' },
];

for (const theme of ['dark', 'light']) {
  test.describe(theme, () => {
    test.use({ viewport: { width: 960, height: 640 } });

    for (const { name, fixture, ready } of steps) {
      test(name, async ({ page }) => {
        await open(page, fixture, theme);
        await expect(page.getByText(ready)).toBeVisible();
        await expect(page).toHaveScreenshot(`onboarding-${name}-${theme}.png`);
      });
    }

    test('try it after the first dictation', async ({ page }) => {
      await open(page, 'try', theme);
      await page.getByRole('textbox', { name: 'Practice' }).fill('Hi, this is my first note with SpeakType. It actually works');
      await expect(page.getByText('That worked.')).toBeVisible();
      await expect(page).toHaveScreenshot(`onboarding-try-it-done-${theme}.png`);
    });

    test('try it comparing another model', async ({ page }) => {
      await open(page, 'try-compare', theme);
      await page.getByRole('textbox', { name: 'Practice' }).fill('Hi, this is my first note with SpeakType. It actually works');
      await page.evaluate(() => {
        (window.__fixture.get_settings as { model: string }).model = 'small';
        return window.__mock.emit('settings-changed', {});
      });
      await expect(page.getByRole('button', { name: 'Keep Balanced' })).toBeVisible();
      await expect(page.getByRole('button', { name: 'Back to Fast' })).toBeVisible();
      await expect(page).toHaveScreenshot(`onboarding-try-it-compare-${theme}.png`);
    });

    test('try it with the model menu open', async ({ page }) => {
      await open(page, 'try', theme);
      await page.getByRole('combobox', { name: 'Speech model' }).click();
      await expect(page.locator('[role=listbox]')).toBeVisible();
      await expect(page).toHaveScreenshot(`onboarding-try-it-menu-${theme}.png`);
    });
  });
}

test('the speech model slider is pre-set to Balanced and does not requeue a running download', async ({ page }) => {
  await page.setViewportSize({ width: 960, height: 640 });
  await open(page, 'model', 'dark');
  await expect(page.getByRole('slider')).toHaveAttribute('aria-valuetext', 'Balanced');
  const queued = await page.evaluate(() => window.__mock.calls.filter((call) => call.cmd === 'load_model'));
  expect(queued).toEqual([]);
});
