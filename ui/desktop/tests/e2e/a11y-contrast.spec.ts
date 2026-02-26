import { test, expect } from './fixtures';
import AxeBuilder from '@axe-core/playwright';

test.skip(
  process.env.RUN_A11Y_CONTRAST !== 'true',
  'Set RUN_A11Y_CONTRAST=true to enable this e2e contrast audit'
);

async function runContrastAudit(page: import('@playwright/test').Page, label: string, testInfo: import('@playwright/test').TestInfo) {
  const results = await new AxeBuilder({ page })
    .withRules(['color-contrast'])
    .analyze();

  await testInfo.attach(`axe-contrast-${label}.json`, {
    body: JSON.stringify(results, null, 2),
    contentType: 'application/json',
  });

  expect(results.violations, `[${label}] ${JSON.stringify(results.violations, null, 2)}`).toEqual([]);
}

async function navigateSidebar(page: import('@playwright/test').Page, itemLabel: string) {
  const testId = `sidebar-${itemLabel.toLowerCase()}-button`;
  const locator = page.locator(`[data-testid="${testId}"]`);
  try {
    await locator.waitFor({ state: 'visible', timeout: 60_000 });
    await locator.click();
    return true;
  } catch {
    return false;
  }
}

test('a11y: color contrast (axe-core)', async ({ goosePage }, testInfo) => {
  await goosePage.waitForLoadState('domcontentloaded');

  // Ensure the app has rendered.
  await goosePage.waitForFunction(() => {
    const root = document.getElementById('root');
    return root && root.children.length > 0;
  });

  // If the sidebar isn't present (e.g. first run / provider not configured),
  // run a minimal audit on the initial screen.
  const hasSidebar = (await goosePage.locator('[data-testid^="sidebar-"]').count()) > 0;
  if (!hasSidebar) {
    await runContrastAudit(goosePage, 'initial-light', testInfo);

    await goosePage.evaluate(() => {
      document.documentElement.classList.add('dark');
    });
    await goosePage.waitForTimeout(250);
    await runContrastAudit(goosePage, 'initial-dark', testInfo);
    return;
  }

  // --- Chat ---
  await navigateSidebar(goosePage, 'home');
  await goosePage.waitForSelector('[data-testid="chat-input"]', { timeout: 30_000 });
  await runContrastAudit(goosePage, 'chat-light', testInfo);

  // --- Settings ---
  await navigateSidebar(goosePage, 'settings');
  await goosePage.waitForSelector('[data-testid="settings-app-tab"]', { timeout: 30_000 });
  await runContrastAudit(goosePage, 'settings-light', testInfo);

  // --- Monitoring ---
  await navigateSidebar(goosePage, 'monitoring');
  await goosePage.waitForSelector('text=Monitoring', { timeout: 30_000 });
  await runContrastAudit(goosePage, 'monitoring-light', testInfo);

  // --- Evaluate ---
  await navigateSidebar(goosePage, 'evaluate');
  await goosePage.waitForSelector('text=Evaluate', { timeout: 30_000 });
  await runContrastAudit(goosePage, 'evaluate-light', testInfo);

  // --- Extensions ---
  await navigateSidebar(goosePage, 'extensions');
  await goosePage.waitForSelector('text=Extensions', { timeout: 30_000 });
  await runContrastAudit(goosePage, 'extensions-light', testInfo);

  // Dark-mode pass: toggle via the class to exercise the dark token palette.
  await goosePage.evaluate(() => {
    document.documentElement.classList.add('dark');
  });
  await goosePage.waitForTimeout(250);

  await navigateSidebar(goosePage, 'home');
  await goosePage.waitForSelector('[data-testid="chat-input"]', { timeout: 30_000 });
  await runContrastAudit(goosePage, 'chat-dark', testInfo);

  await navigateSidebar(goosePage, 'settings');
  await goosePage.waitForSelector('[data-testid="settings-app-tab"]', { timeout: 30_000 });
  await runContrastAudit(goosePage, 'settings-dark', testInfo);
});
