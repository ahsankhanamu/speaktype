import { readFileSync } from 'node:fs';
import { expect, test, type Page } from '@playwright/test';

const registry = readFileSync(new URL('../../gallery/registry.ts', import.meta.url), 'utf8');
const ids = [...registry.matchAll(/id: '([^']+)'/g)].map(([, id]) => id).filter((id) => id !== 'fixtures');

const FORCED: Record<string, string[]> = { hover: ['hover'], pressed: ['hover', 'active'], focus: ['focus', 'focus-visible'] };
const HIGHLIGHT: Record<string, string> = { components: 'Feature request', dropdown: 'Feature request', 'popup-menu': 'Bottom centre' };

async function forceStates(page: Page) {
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('DOM.enable');
  await cdp.send('CSS.enable');
  const { root } = await cdp.send('DOM.getDocument', { depth: 0 });
  for (const [state, classes] of Object.entries(FORCED)) {
    const { nodeIds } = await cdp.send('DOM.querySelectorAll', { nodeId: root.nodeId, selector: `[data-force-state="${state}"]` });
    for (const nodeId of nodeIds) await cdp.send('CSS.forcePseudoState', { nodeId, forcedPseudoClasses: classes });
  }
}

for (const theme of ['light', 'dark'] as const) {
  for (const id of ids) {
    test(`${id} ${theme}`, async ({ page }) => {
      await page.goto(`/#/${id}?theme=${theme}`);
      await page.evaluate(() => document.fonts.ready);
      for (const trigger of await page.locator('[data-screenshot-click] button').all()) {
        await trigger.focus();
        await page.keyboard.press('Enter');
      }
      if (HIGHLIGHT[id]) await page.locator('[role=option], [role=menuitemradio]').filter({ hasText: HIGHLIGHT[id] }).hover();
      else await page.mouse.move(0, 0);
      await forceStates(page);
      await expect(page).toHaveScreenshot(`${id}-${theme}.png`, { fullPage: true, maxDiffPixelRatio: 0.001, animations: 'disabled', caret: 'hide' });
    });
  }
}
