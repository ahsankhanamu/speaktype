import { expect, test, type Page } from '@playwright/test';

test.use({ viewport: { width: 960, height: 720 }, timezoneId: 'UTC', locale: 'en-GB' });

const NOW = new Date('2026-11-19T12:00:00Z');

async function open(page: Page, route: string, theme: string, fixture = 'app-window') {
  await page.clock.setFixedTime(NOW);
  await page.emulateMedia({ colorScheme: theme as 'dark' | 'light' });
  await page.goto(`/tests/visual/harness.html?scene=app-window&fixture=${fixture}&theme=${theme}#/${route}`);
  await expect(page.locator('main')).not.toBeEmpty();
}

const calls = (page: Page) => page.evaluate(() => window.__mock.calls.map((call) => call.cmd));

for (const theme of ['dark', 'light']) {
  test.describe(theme, () => {
    for (const route of ['general', 'appearance', 'sessions', 'storage', 'advanced', 'about']) {
      test(route, async ({ page }) => {
        await open(page, route, theme);
        await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
        if (route === 'advanced') await expect(page.getByText('Running on port 8002')).toBeVisible();
        if (route === 'storage') await expect(page.getByText('650 MB')).toBeVisible();
        await expect(page).toHaveScreenshot(`app-window-${route}-${theme}.png`);
      });
    }

    test('dictation', async ({ page }) => {
      await open(page, 'dictation', theme);
      await expect(page.getByText('Accurate is downloading')).toBeVisible();
      await expect(page).toHaveScreenshot(`app-window-dictation-${theme}.png`);
    });

    test('providers', async ({ page }) => {
      await open(page, 'providers', theme);
      await expect(page.getByText('Connected · 6 speech models found')).toBeVisible();
      await page.getByRole('button', { name: 'Test', exact: true }).click();
      await expect(page.getByText('Works · 0.4 s for a 3 s test clip')).toBeVisible();
      await expect(page).toHaveScreenshot(`app-window-providers-${theme}.png`);
    });

    test('permissions with tests running', async ({ page }) => {
      await open(page, 'permissions', theme);
      await expect(page.getByRole('button', { name: 'Test', exact: true })).toBeVisible();
      await expect(page.getByRole('button', { name: 'Stop test' })).toHaveCount(0);
      expect(await calls(page)).not.toContain('start_mic_test');
      await expect(page).toHaveScreenshot(`app-window-permissions-idle-${theme}.png`);
      await page.getByRole('button', { name: 'Test', exact: true }).click();
      await page.evaluate(() => window.__mock.emit('mictest:level', { rms: 0.03, peak: 0.1, clipped: false }));
      await expect(page.getByText('Say something… hearing you on MacBook Pro Microphone')).toBeVisible();
      await expect(page).toHaveScreenshot(`app-window-permissions-mic-${theme}.png`);
    });

    test('history', async ({ page }) => {
      await open(page, 'history', theme);
      await expect(page.getByRole('heading', { name: 'Today' })).toBeVisible();
      await expect(page.getByRole('button', { name: 'Copy', exact: true })).toBeVisible();
      await expect(page).toHaveScreenshot(`app-window-history-${theme}.png`);
    });

    test('history, nothing yet', async ({ page }) => {
      await page.route('**/fixtures/app-window-empty.json', async (route) => {
        const base = await (await page.request.get(route.request().url().replace('-empty', ''))).json();
        await route.fulfill({ json: { ...base, get_history: { entries: [] }, session_list: [] } });
      });
      await open(page, 'history', theme, 'app-window-empty');
      await expect(page.getByText('Nothing here yet')).toBeVisible();
      await expect(page).toHaveScreenshot(`app-window-history-empty-${theme}.png`);
    });
  });
}

test('Search finds an individual setting and highlights its row', async ({ page }) => {
  await open(page, 'general', 'dark');
  await page.getByRole('searchbox').fill('draw');
  await page.getByRole('button', { name: /Sessions › Draw while holding/ }).click();
  await expect(page.getByRole('heading', { name: 'Sessions', level: 1 })).toBeVisible();
  await expect(page.locator('.st-row[data-match]')).toContainText('Draw while holding');
});

test('The window opens on the page it was last on', async ({ page }) => {
  await open(page, 'storage', 'dark');
  await page.goto('/tests/visual/harness.html?scene=app-window&fixture=app-window&theme=dark');
  await expect(page.getByRole('heading', { name: 'Storage', level: 1 })).toBeVisible();
});

test('1,000 history entries render a window of rows, not all of them', async ({ page }) => {
  await page.route('**/fixtures/app-window-many.json', async (route) => {
    const base = await (await page.request.get(route.request().url().replace('-many', ''))).json();
    const entries = Array.from({ length: 1000 }, (_, i) => ({
      id: String(i),
      text: `Dictation number ${i} with a few words in it`,
      timestamp: new Date(NOW.getTime() - i * 3_600_000).toISOString(),
      audio_file: null,
      duration_secs: 5,
      outcome: 'pasted',
    }));
    await route.fulfill({ json: { ...base, get_history: { entries }, session_list: [] } });
  });
  await open(page, 'history', 'dark', 'app-window-many');
  await expect(page.getByRole('button', { name: /Dictation number 0 with/ })).toBeVisible();
  expect(await page.locator('[role=listitem]').count()).toBeLessThan(40);
  await page.getByRole('list', { name: 'History' }).evaluate((el) => (el.scrollTop = 40_000));
  await expect(page.getByRole('button', { name: /Dictation number 6\d\d with/ }).first()).toBeVisible();
  expect(await page.locator('[role=listitem]').count()).toBeLessThan(40);
});
