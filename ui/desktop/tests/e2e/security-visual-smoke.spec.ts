import { Page } from '@playwright/test';
import { test, expect } from './fixtures';

function getAllPages(goosePage: Page): Page[] {
  const browser = goosePage.context().browser();
  if (!browser) {
    return goosePage.context().pages();
  }

  return browser.contexts().flatMap((context) => context.pages());
}

async function waitForNewWindow(goosePage: Page, previousCount: number): Promise<Page> {
  await expect
    .poll(() => getAllPages(goosePage).length, {
      message: 'Expected a new Electron window to open',
      timeout: 15000,
    })
    .toBeGreaterThan(previousCount);

  const newPages = getAllPages(goosePage).slice(previousCount);
  const newPage = newPages[newPages.length - 1];
  await newPage.waitForLoadState('domcontentloaded');
  await expect(newPage.locator('[data-testid="chat-input"]:visible').first()).toBeVisible({
    timeout: 30000,
  });
  return newPage;
}

test('security launcher and recipes starters stay visible and clickable in repo preview', async ({
  goosePage,
}) => {
  await goosePage.waitForSelector('[data-testid="chat-input"]', { timeout: 30000 });

  await goosePage.getByRole('button', { name: '配方' }).click();

  const recipesSection = goosePage.getByTestId('recipes-security-tasks');
  await expect(recipesSection).toBeVisible({ timeout: 30000 });
  await expect(recipesSection).toContainText('安全任务入口');
  await expect(goosePage.getByTestId('recipes-security-task-vuln-triage')).toBeVisible();
  await expect(goosePage.getByTestId('recipes-security-task-ioc-analysis')).toBeVisible();
  await expect(goosePage.getByTestId('recipes-security-task-badge-vuln-triage')).toContainText(
    'Recipe'
  );
  await expect(goosePage.getByTestId('recipes-security-task-badge-ioc-analysis')).toContainText(
    '预览'
  );
  await expect(goosePage.getByTestId('security-extension-status-threat-intel-mcp')).toBeVisible();

  const beforeRecipeWindowCount = getAllPages(goosePage).length;
  await goosePage.getByTestId('recipes-security-task-secondary-vuln-triage').click();
  const recipeWindow = await waitForNewWindow(goosePage, beforeRecipeWindowCount);
  await expect(recipeWindow).toHaveTitle(/Security Goose/i);

  await goosePage.bringToFront();

  await goosePage.getByTestId('recipes-security-task-primary-ioc-analysis').click();
  await goosePage.waitForFunction(() => window.location.hash.startsWith('#/pair'));
  await expect(goosePage.locator('[data-testid="chat-input"]:visible').first()).toBeVisible({
    timeout: 30000,
  });

  await goosePage.evaluate(() => {
    window.location.hash = '#/launcher';
  });
  await goosePage.waitForFunction(() => window.location.hash === '#/launcher');

  const launcherSection = goosePage.getByTestId('launcher-security-tasks');
  await expect(launcherSection).toBeVisible();
  await expect(launcherSection).toContainText('安全任务入口');
  await expect(goosePage.getByTestId('launcher-security-task-vuln-triage')).toBeVisible();
  await expect(goosePage.getByTestId('launcher-security-task-wooyun-legacy')).toBeVisible();
  await expect(goosePage.getByTestId('security-extension-status-browser-assist-mcp')).toBeVisible();
});
