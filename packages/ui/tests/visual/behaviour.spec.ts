import { expect, test } from '@playwright/test';

test.beforeEach(async ({ page }) => {
  await page.goto('/#/fixtures?theme=dark');
});

test('select opens item-aligned and stays inside the viewport near the bottom edge', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 640 });
  await page.getByTestId('edge-trigger').click();
  const content = page.getByRole('listbox');
  await expect(content).toBeVisible();
  const box = (await content.boundingBox())!;
  expect(box.y).toBeGreaterThanOrEqual(8 - 0.5);
  expect(box.y + box.height).toBeLessThanOrEqual(632 + 0.5);
  const selected = (await page.getByRole('option', { selected: true }).locator('[data-st-label]').boundingBox())!;
  const trigger = (await page.getByTestId('edge-trigger').locator('[data-st-label]').boundingBox())!;
  const clamped = Math.abs(box.y - 8) < 1 || Math.abs(box.y + box.height - 632) < 1;
  expect(clamped || Math.abs(selected.y - trigger.y) <= 1).toBe(true);
});

test('select opens with the selected label exactly over the trigger label when there is room', async ({ page }) => {
  await page.getByTestId('open-sheet').click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await page.waitForTimeout(300);
  await page.getByTestId('sheet-trigger').click();
  const selected = (await page.getByRole('option', { selected: true }).locator('[data-st-label]').boundingBox())!;
  const trigger = (await page.getByTestId('sheet-trigger').locator('[data-st-label]').boundingBox())!;
  expect(Math.abs(selected.y - trigger.y)).toBeLessThanOrEqual(1);
  expect(Math.abs(selected.x - trigger.x)).toBeLessThanOrEqual(1);
});

test('select inside a sheet renders above the sheet', async ({ page }) => {
  await page.getByTestId('open-sheet').click();
  await page.getByTestId('sheet-trigger').click();
  const content = page.getByRole('listbox');
  await expect(content).toBeVisible();
  const above = await content.evaluate((el) => {
    const { x, y, width, height } = el.getBoundingClientRect();
    const hit = document.elementFromPoint(x + width / 2, y + height / 2);
    return !!hit && el.contains(hit) && el.closest('#st-layers') !== null;
  });
  expect(above).toBe(true);
});

test('long path middle-truncates and shows a tooltip only when truncated', async ({ page }) => {
  const long = page.getByTestId('long-path');
  const text = (await long.textContent())!;
  expect(text).toMatch(/^\/Users.*….*\.png$/);
  await long.hover();
  await page.waitForTimeout(600);
  await expect(page.getByRole('tooltip')).toContainText('/Users/someone/Library/Application Support/SpeakType/Sessions/2026-11-19-checkout-bug/images/03-mark.png');

  await page.mouse.move(0, 0);
  await expect(page.getByRole('tooltip')).toHaveCount(0);
  await page.getByTestId('short-path').hover();
  await page.waitForTimeout(600);
  await expect(page.getByRole('tooltip')).toHaveCount(0);
});

test('tail-truncated text ends in an ellipsis and keeps its full value in the tooltip', async ({ page }) => {
  const overflow = await page.getByTestId('long-text').evaluate((el) => el.scrollWidth > el.clientWidth);
  expect(overflow).toBe(true);
  await page.getByTestId('long-text').hover();
  await page.waitForTimeout(600);
  await expect(page.getByRole('tooltip')).toContainText('does not fit into its row');
});

test('sheet toggled 10× in 1 s ends closed with focus restored and no inert leftovers', async ({ page }) => {
  const opener = page.getByTestId('toggle-sheet');
  const started = Date.now();
  for (let i = 0; i < 10; i++) {
    await opener.click();
    await page.keyboard.press('Escape');
  }
  expect(Date.now() - started).toBeLessThan(5000);
  await expect(page.locator('[data-dialog-overlay]')).toHaveCount(0);
  await expect(opener).toBeFocused();
  const leftovers = await page.evaluate(() => ({
    inert: document.querySelectorAll('[inert]').length,
    bodyPointerEvents: getComputedStyle(document.body).pointerEvents,
  }));
  expect(leftovers).toEqual({ inert: 0, bodyPointerEvents: 'auto' });
});

test('a busy button keeps its width', async ({ page }) => {
  const idle = (await page.getByTestId('idle-button').boundingBox())!;
  const busy = (await page.getByTestId('busy-button').boundingBox())!;
  expect(busy.width).toBe(idle.width);
});

test('rows have the info, control and icon heights', async ({ page }) => {
  const heights = await page.locator('.st-row').evaluateAll((rows) => rows.map((row) => row.getBoundingClientRect().height));
  expect(heights[0]).toBe(32);
  expect(heights[1]).toBe(40);
  expect(heights[2]).toBeGreaterThanOrEqual(52);
});

test('pop-up menu opens over its button and picks an item with the pointer', async ({ page }) => {
  await page.goto('/#/popup-menu?theme=dark');
  const trigger = page.locator('button.st-trigger');
  await trigger.click();
  await expect(page.getByRole('menu')).toBeVisible();
  const current = (await page.getByRole('menuitemradio', { checked: true }).locator('[data-st-label]').boundingBox())!;
  const label = (await trigger.locator('[data-st-label]').boundingBox())!;
  expect(Math.abs(current.y - label.y)).toBeLessThanOrEqual(1);
  await page.getByRole('menuitemradio', { name: 'Top right' }).click();
  await expect(trigger).toContainText('Top right');
  await expect(page.getByRole('menu')).toHaveCount(0);
});

test('every button variant draws a focus ring on keyboard focus', async ({ page }) => {
  for (const variant of ['primary', 'secondary', 'plain', 'destructive']) {
    const button = page.getByTestId(`variant-${variant}`);
    await button.focus();
    const shadow = await button.evaluate((el) => getComputedStyle(el).boxShadow);
    expect(shadow, variant).not.toBe('none');
    expect(shadow, variant).toContain('rgba(10, 132, 255, 0.85)');
  }
});

test('aligned pop-ups are at least as wide as their button', async ({ page }) => {
  await page.getByTestId('open-sheet').click();
  await page.waitForTimeout(300);
  const trigger = page.getByTestId('sheet-trigger');
  const triggerWidth = (await trigger.boundingBox())!.width;
  await trigger.click();
  expect((await page.getByRole('listbox').boundingBox())!.width).toBeGreaterThanOrEqual(triggerWidth);

  await page.goto('/#/popup-menu?theme=dark');
  const popup = page.locator('button.st-trigger');
  const popupWidth = (await popup.boundingBox())!.width;
  await popup.click();
  await expect(page.getByRole('menu')).toBeVisible();
  expect((await page.getByRole('menu').boundingBox())!.width).toBeGreaterThanOrEqual(popupWidth);
});

test('clickable controls show the pointing hand; disabled ones and text fields do not', async ({ page }) => {
  const cursor = (selector: string) => page.locator(selector).first().evaluate((el) => getComputedStyle(el).cursor);
  expect(await cursor('button:not(:disabled)')).toBe('pointer');
  const disabled = page.locator('button:disabled');
  if (await disabled.count()) expect(await disabled.first().evaluate((el) => getComputedStyle(el).cursor)).not.toBe('pointer');
  const field = page.locator('.st-field').first();
  if (await field.count()) expect(await field.evaluate((el) => getComputedStyle(el).cursor)).not.toBe('pointer');
});
