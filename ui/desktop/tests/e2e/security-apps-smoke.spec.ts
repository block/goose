import { Page } from '@playwright/test';
import { test, expect } from './fixtures';

function getAllPages(goosePage: Page): Page[] {
  const browser = goosePage.context().browser();
  if (!browser) {
    return goosePage.context().pages();
  }

  return browser.contexts().flatMap((context) => context.pages());
}

async function waitForSecretScannerWindow(goosePage: Page, previousCount: number): Promise<Page> {
  await expect
    .poll(() => getAllPages(goosePage).length, {
      message: 'Expected a new app window to open',
      timeout: 15000,
    })
    .toBeGreaterThan(previousCount);

  const deadline = Date.now() + 15000;

  while (Date.now() < deadline) {
    for (const page of getAllPages(goosePage)) {
      if (page.isClosed()) {
        continue;
      }

      try {
        await page.waitForLoadState('domcontentloaded', { timeout: 1000 });
      } catch {
        continue;
      }

      const title = await page.title().catch(() => '');
      const hasDemoButton = await page
        .locator('#loadDemoBtn')
        .count()
        .then((count) => count > 0)
        .catch(() => false);

      if (/Secret(?:\s*&\s*|\s+)Credential Scanner/i.test(title) || hasDemoButton) {
        return page;
      }
    }

    await new Promise((resolve) => setTimeout(resolve, 250));
  }

  const titles = await Promise.all(
    getAllPages(goosePage).map((page) => page.title().catch(() => '<unavailable>'))
  );
  throw new Error(`Secret Credential Scanner window not found. Observed titles: ${titles.join(' | ')}`);
}

async function waitForAppWindow(
  goosePage: Page,
  previousCount: number,
  titlePattern: RegExp,
  description: string
): Promise<Page> {
  await expect
    .poll(() => getAllPages(goosePage).length, {
      message: `Expected a new ${description} window to open`,
      timeout: 15000,
    })
    .toBeGreaterThan(previousCount);

  const deadline = Date.now() + 15000;

  while (Date.now() < deadline) {
    for (const page of getAllPages(goosePage)) {
      if (page.isClosed()) {
        continue;
      }

      try {
        await page.waitForLoadState('domcontentloaded', { timeout: 1000 });
      } catch {
        continue;
      }

      const title = await page.title().catch(() => '');
      if (titlePattern.test(title)) {
        return page;
      }
    }

    await new Promise((resolve) => setTimeout(resolve, 250));
  }

  const titles = await Promise.all(
    getAllPages(goosePage).map((page) => page.title().catch(() => '<unavailable>'))
  );
  throw new Error(`${description} window not found. Observed titles: ${titles.join(' | ')}`);
}

test('security built-in apps stay visible and launchable in repo preview', async ({ goosePage }) => {
  await goosePage.waitForSelector('[data-testid="chat-input"]', { timeout: 30000 });

  const marker = `SECURITY_APPS_READY_${Date.now()}`;
  const chatInput = goosePage.locator('[data-testid="chat-input"]');
  await chatInput.fill(`Reply with exactly: ${marker}`);
  await chatInput.press('Enter');

  await goosePage.waitForSelector('[data-testid="loading-indicator"]', {
    state: 'visible',
    timeout: 10000,
  });
  await goosePage.waitForSelector('[data-testid="loading-indicator"]', {
    state: 'hidden',
    timeout: 120000,
  });

  await expect(goosePage.locator('[data-testid="message-container"]').last()).toContainText(marker, {
    timeout: 30000,
  });

  await goosePage.evaluate(() => {
    window.location.hash = '#/apps';
  });
  await goosePage.waitForFunction(() => window.location.hash === '#/apps');

  await expect(goosePage.getByTestId('apps-built-in-security-section')).toBeVisible();
  await expect(goosePage.getByTestId('apps-card-ioc-toolbox')).toBeVisible();
  await expect(goosePage.getByTestId('apps-card-encode-hash-lab')).toBeVisible();
  await expect(goosePage.getByTestId('apps-card-secret-credential-scanner')).toBeVisible();
  await expect(goosePage.getByTestId('apps-card-jwt-inspector')).toBeVisible();
  await expect(goosePage.getByTestId('apps-card-clock')).toHaveCount(0);
  await expect(goosePage.getByTestId('apps-card-chat')).toHaveCount(0);

  const beforeWindowCount = getAllPages(goosePage).length;
  await goosePage.getByTestId('apps-launch-secret-credential-scanner').click();
  const toolWindow = await waitForSecretScannerWindow(goosePage, beforeWindowCount);
  await expect(toolWindow).toHaveTitle(/Secret(?:\s*&\s*|\s+)Credential Scanner/i);
  await expect(toolWindow.locator('iframe')).toBeVisible();

  const sandboxFrame = toolWindow.frameLocator('iframe').first();
  await expect(sandboxFrame.locator('iframe')).toBeVisible({ timeout: 15000 });

  const toolFrame = sandboxFrame.frameLocator('iframe').first();
  await expect(toolFrame.locator('#loadDemoBtn')).toBeVisible({ timeout: 15000 });
  await toolFrame.locator('#loadDemoBtn').click();
  await expect(toolFrame.getByText('处理建议')).toHaveCount(0);
  await expect(toolFrame.getByText('复制结构化 JSON')).toBeVisible();
  await expect(toolFrame.locator('.result-section').first()).toBeVisible();
  await expect(toolFrame.locator('.panel')).toHaveCount(0);
  await expect(toolFrame.locator('.summary-card')).toHaveCount(0);

  const beforeEncodeWindowCount = getAllPages(goosePage).length;
  await goosePage.getByTestId('apps-launch-encode-hash-lab').click();
  const encodeWindow = await waitForAppWindow(
    goosePage,
    beforeEncodeWindowCount,
    /Encode.*Hash Lab/i,
    'Encode & Hash Lab'
  );
  await expect(encodeWindow).toHaveTitle(/Encode.*Hash Lab/i);
  await expect(encodeWindow.locator('iframe')).toBeVisible();

  const encodeSandbox = encodeWindow.frameLocator('iframe').first();
  await expect(encodeSandbox.locator('iframe')).toBeVisible({ timeout: 15000 });

  const encodeFrame = encodeSandbox.frameLocator('iframe').first();
  await expect(encodeFrame.locator('#plainInput')).toBeVisible({ timeout: 15000 });
  await encodeFrame.locator('#plainInput').fill('hello security goose');
  await encodeFrame.locator('#operationSelect').selectOption('base64-encode');
  await encodeFrame.locator('#addStepBtn').click();
  await encodeFrame.locator('#operationSelect').selectOption('url-encode');
  await encodeFrame.locator('#addStepBtn').click();
  await encodeFrame.locator('#runPipelineBtn').click();

  await expect(encodeFrame.locator('#status')).toContainText('操作链执行完成，共处理 2 步。');
  await expect(encodeFrame.locator('[data-step-title]').first()).toContainText('Base64 编码');
  await expect(encodeFrame.locator('[data-step-title]').nth(1)).toContainText('URL 编码');
  await expect(encodeFrame.locator('#finalOutput')).toHaveValue(/aGVsbG8gc2VjdXJpdHkgZ29vc2U%3D/);

  const beforeJwtWindowCount = getAllPages(goosePage).length;
  await goosePage.getByTestId('apps-launch-jwt-inspector').click();
  const jwtWindow = await waitForAppWindow(
    goosePage,
    beforeJwtWindowCount,
    /JWT Inspector/i,
    'JWT Inspector'
  );
  await expect(jwtWindow).toHaveTitle(/JWT Inspector/i);
  await expect(jwtWindow.locator('iframe')).toBeVisible();

  const jwtSandbox = jwtWindow.frameLocator('iframe').first();
  await expect(jwtSandbox.locator('iframe')).toBeVisible({ timeout: 15000 });

  const jwtFrame = jwtSandbox.frameLocator('iframe').first();
  await expect(jwtFrame.locator('#loadDemoBtn')).toBeVisible({ timeout: 15000 });
  await jwtFrame.locator('#loadDemoBtn').click();
  await expect(jwtFrame.getByText('签名状态')).toBeVisible();
  await expect(jwtFrame.getByText('风险提示')).toBeVisible();
  await expect(jwtFrame.locator('[data-copy="signature"]')).toBeVisible();
  await expect(jwtFrame.locator('.panel')).toHaveCount(0);
  await expect(jwtFrame.locator('.card')).toHaveCount(0);
});
