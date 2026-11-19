import { expect, test, type Page } from '@playwright/test';

const open = (page: Page, scene: string, fixture: string, theme: string) =>
  page.goto(`/tests/visual/harness.html?scene=${scene}&fixture=${fixture}&theme=${theme}`);

for (const theme of ['dark', 'light']) {
  test.describe(theme, () => {
    test('model menu', async ({ page }) => {
      await page.setViewportSize({ width: 560, height: 560 });
      await open(page, 'menu', 'models-onboarding', theme);
      await page.getByRole('combobox', { name: 'Speech model' }).first().click();
      await expect(page.getByRole('option', { name: /Balanced/ })).toContainText('Downloading · 64%');
      await expect(page.locator('[role=listbox]')).toBeVisible();
      const top = async (selector: string) => (await page.locator(selector).first().boundingBox())!.y;
      await expect
        .poll(async () => Math.abs((await top('[role=option][data-selected] [data-st-label]')) - (await top('.model-trigger.row [data-st-label]'))))
        .toBeLessThanOrEqual(2);
      await expect(page).toHaveScreenshot(`model-menu-${theme}.png`);
    });

    test('speech models sheet', async ({ page }) => {
      await open(page, 'sheet', 'models-board', theme);
      await expect(page.getByRole('dialog')).toContainText('580 MB of 1.5 GB · 2 min');
      await expect(page.getByRole('dialog')).toContainText('Connection lost');
      await expect(page).toHaveScreenshot(`models-sheet-${theme}.png`);
    });

    test('mic picker', async ({ page }) => {
      await page.setViewportSize({ width: 660, height: 300 });
      await open(page, 'mic', 'mic-board', theme);
      await expect(page.getByRole('meter').nth(1)).toHaveAttribute('aria-valuenow', '0.45');
      await expect(page).toHaveScreenshot(`mic-picker-${theme}.png`);
    });

    test('mic picker with a 300-character name', async ({ page }) => {
      await page.setViewportSize({ width: 660, height: 260 });
      await open(page, 'mic', 'mic-edge', theme);
      await expect(page.getByRole('radio')).toHaveCount(3);
      await expect(page).toHaveScreenshot(`mic-picker-edge-${theme}.png`);
    });
  });
}

test('mic levels start with the picker', async ({ page }) => {
  await open(page, 'mic', 'mic-board', 'dark');
  await expect(page.getByRole('radio')).toHaveCount(4);
  const commands = await page.evaluate(() => window.__mock.calls.map((call) => call.cmd));
  expect(commands).toContain('mic_levels_start');
});
