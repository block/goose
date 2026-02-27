import axe from 'axe-core';
import type { Page, TestInfo } from '@playwright/test';
import { expect, test } from './fixtures';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname } from 'node:path';

test.skip(
  process.env.RUN_A11Y_CONTRAST !== 'true',
  'Set RUN_A11Y_CONTRAST=true to enable this e2e contrast audit'
);

// This suite can be slow on first run (starting goosed, reading user config, etc.).
// We prefer multiple smaller tests for better reporting.
// Each test still launches a fresh Electron app (fixture behavior).
// If this becomes too slow, we can introduce a worker-scoped Electron fixture.

test.describe.configure({ timeout: 180_000 });

type AxeRunResult = {
  violations: Array<{
    id: string;
    description: string;
    help: string;
    impact: string | null;
    nodes: unknown[];
  }>;
};

async function ensureRootReady(page: Page) {
  await page.waitForLoadState('domcontentloaded');
  await page.waitForFunction(
    () => {
      const root = document.getElementById('root');
      return root && root.children.length > 0;
    },
    undefined,
    { timeout: 30_000 }
  );
}

async function ensureAxeInjected(page: Page) {
  const hasAxe = await page
    .evaluate(() => Boolean((window as unknown as { axe?: unknown }).axe))
    .catch(() => false);
  if (hasAxe) return;

  await page.addScriptTag({ content: axe.source });

  await page.waitForFunction(
    () => Boolean((window as unknown as { axe?: unknown }).axe),
    undefined,
    { timeout: 5_000 }
  );
}

async function runContrastAudit(page: Page, label: string, testInfo: TestInfo) {
  await ensureAxeInjected(page);

  const results = await page.evaluate(async () => {
    const axeApi = (window as unknown as { axe: { run: (ctx: unknown, opts: unknown) => Promise<unknown> } }).axe;
    return (await axeApi.run(document, {
      runOnly: {
        type: 'rule',
        values: ['color-contrast'],
      },
    })) as AxeRunResult;
  });

  await testInfo.attach(`axe-contrast-${label}.json`, {
    body: JSON.stringify(results, null, 2),
    contentType: 'application/json',
  });

  // Persist results to disk for easy inspection (even when the test fails).
  const outPath = testInfo.outputPath(`axe-contrast-${label}.json`);
  await mkdir(dirname(outPath), { recursive: true });
  await writeFile(outPath, JSON.stringify(results, null, 2));

  // eslint-disable-next-line no-console
  console.log(`[axe-contrast] ${label}: ${results.violations.length} violation(s) -> ${outPath}`);

  expect(
    results.violations.length,
    `[${label}] Found ${results.violations.length} contrast violation(s). See attached axe-contrast-${label}.json`
  ).toBe(0);
}

async function navigateSidebar(page: Page, itemLabel: string): Promise<boolean> {
  const testId = `sidebar-${itemLabel.toLowerCase()}-button`;
  const locator = page.locator(`[data-testid="${testId}"]`);

  try {
    await locator.waitFor({ state: 'visible', timeout: 2_000 });
    await locator.click({ timeout: 2_000 });
    return true;
  } catch {
    return false;
  }
}

async function clickTabIfPresent(page: Page, tabName: RegExp): Promise<boolean> {
  const tab = page.getByRole('button', { name: tabName }).first();
  if ((await tab.count()) === 0) return false;

  try {
    await tab.click({ timeout: 2_000 });
    return true;
  } catch {
    return false;
  }
}

function hashRouteUrl(page: Page, route: string) {
  const base = page.url().split('#')[0];
  const normalized = route.startsWith('/') ? route : `/${route}`;
  return `${base}#${normalized}`;
}

async function gotoHashRouteOrWelcome(page: Page, route: string): Promise<'ok' | 'welcome'> {
  const url = hashRouteUrl(page, route);
  await page.goto(url);
  await page.waitForURL(/#\/(welcome|monitoring|evaluate|pair|settings|extensions|configure-providers)/i, {
    timeout: 30_000,
  });

  return /#\/welcome\b/i.test(page.url()) ? 'welcome' : 'ok';
}

async function enableDarkMode(page: Page) {
  await page.evaluate(() => {
    document.documentElement.classList.add('dark');
  });
  await page.waitForTimeout(250);
}

async function dismissOptionalChooseModelModal(page: Page) {
  const selectModelButton = page.getByRole('button', { name: /select model/i });
  if (await selectModelButton.isVisible().catch(() => false)) {
    await selectModelButton.click({ timeout: 10_000 }).catch(() => {});
  }
}

async function dismissOptionalTelemetryModal(page: Page) {
  // Some first-run flows show a telemetry consent dialog.
  const noThanks = page.getByRole('button', { name: /no thanks/i });
  if (await noThanks.isVisible().catch(() => false)) {
    await noThanks.click({ timeout: 10_000 }).catch(() => {});
    return;
  }

  const dialogNoThanks = page
    .getByRole('dialog')
    .filter({ hasText: /telemetry/i })
    .getByRole('button', { name: /no thanks/i });

  if (await dialogNoThanks.isVisible().catch(() => false)) {
    await dialogNoThanks.click({ timeout: 10_000 }).catch(() => {});
  }
}

async function bootstrap(page: Page) {
  await ensureRootReady(page);
  await dismissOptionalChooseModelModal(page);
  await dismissOptionalTelemetryModal(page);
}

test.describe('a11y: color contrast (axe-core)', () => {
  test('initial (light)', async ({ goosePage }, testInfo) => {
    await bootstrap(goosePage);
    await runContrastAudit(goosePage, 'initial-light', testInfo);
  });

  test('chat (light)', async ({ goosePage }, testInfo) => {
    await bootstrap(goosePage);

    const navigated = await navigateSidebar(goosePage, 'home');
    if (!navigated) test.skip(true, 'Sidebar navigation not available in this environment');

    const hasChat = await goosePage.locator('[data-testid="chat-input"]').isVisible().catch(() => false);
    if (!hasChat) test.skip(true, 'Chat input not visible');

    await runContrastAudit(goosePage, 'chat-light', testInfo);
  });

  test('settings (light)', async ({ goosePage }, testInfo) => {
    await bootstrap(goosePage);

    if ((await gotoHashRouteOrWelcome(goosePage, '/settings')) === 'welcome') {
      test.skip(true, 'Requires a configured provider (otherwise app is on /welcome)');
    }

    const settingsTabs = await goosePage.locator('[data-testid="settings-app-tab"]').isVisible().catch(() => false);
    if (!settingsTabs) test.skip(true, 'Settings view not ready');

    await runContrastAudit(goosePage, 'settings-light', testInfo);
  });

  test('monitoring: dashboard (light)', async ({ goosePage }, testInfo) => {
    await bootstrap(goosePage);

    if ((await gotoHashRouteOrWelcome(goosePage, '/monitoring')) === 'welcome') {
      test.skip(true, 'Requires a configured provider (otherwise app is on /welcome)');
    }

    const title = await goosePage.locator('text=Monitoring').isVisible().catch(() => false);
    if (!title) test.skip(true, 'Monitoring view not ready');

    await runContrastAudit(goosePage, 'monitoring-light', testInfo);
  });

  test('monitoring: live tab (light)', async ({ goosePage }, testInfo) => {
    await bootstrap(goosePage);

    if ((await gotoHashRouteOrWelcome(goosePage, '/monitoring')) === 'welcome') {
      test.skip(true, 'Requires a configured provider (otherwise app is on /welcome)');
    }

    if (!(await clickTabIfPresent(goosePage, /^live$/i))) {
      test.skip(true, 'Monitoring Live tab not present');
    }

    await goosePage.waitForTimeout(150);

    const liveHeading = await goosePage
      .getByRole('heading', { name: /live monitoring/i })
      .first()
      .isVisible()
      .catch(() => false);

    if (!liveHeading) test.skip(true, 'Live tab not ready');

    await runContrastAudit(goosePage, 'monitoring-live-light', testInfo);
  });

  test('monitoring: tool analytics tab (light)', async ({ goosePage }, testInfo) => {
    await bootstrap(goosePage);

    if ((await gotoHashRouteOrWelcome(goosePage, '/monitoring')) === 'welcome') {
      test.skip(true, 'Requires a configured provider (otherwise app is on /welcome)');
    }

    if (!(await clickTabIfPresent(goosePage, /^tool analytics$/i))) {
      test.skip(true, 'Tool Analytics tab not present');
    }

    await goosePage.waitForTimeout(150);

    const ready = await goosePage
      .getByText(/daily tool activity|tool usage|failed to load analytics/i)
      .first()
      .isVisible()
      .catch(() => false);

    if (!ready) test.skip(true, 'Tool Analytics view not ready');

    await runContrastAudit(goosePage, 'monitoring-tools-light', testInfo);
  });

  test('evaluate: overview (light)', async ({ goosePage }, testInfo) => {
    await bootstrap(goosePage);

    if ((await gotoHashRouteOrWelcome(goosePage, '/evaluate')) === 'welcome') {
      test.skip(true, 'Requires a configured provider (otherwise app is on /welcome)');
    }

    const title = await goosePage.locator('text=Evaluate').isVisible().catch(() => false);
    if (!title) test.skip(true, 'Evaluate view not ready');

    await runContrastAudit(goosePage, 'evaluate-light', testInfo);
  });

  test('evaluate: datasets tab (light)', async ({ goosePage }, testInfo) => {
    await bootstrap(goosePage);

    if ((await gotoHashRouteOrWelcome(goosePage, '/evaluate')) === 'welcome') {
      test.skip(true, 'Requires a configured provider (otherwise app is on /welcome)');
    }

    if (!(await clickTabIfPresent(goosePage, /^datasets$/i))) {
      test.skip(true, 'Datasets tab not present');
    }

    await goosePage.waitForTimeout(150);

    const heading = await goosePage
      .getByRole('heading', { name: /evaluation datasets/i })
      .first()
      .isVisible()
      .catch(() => false);

    if (!heading) test.skip(true, 'Datasets tab not ready');

    await runContrastAudit(goosePage, 'evaluate-datasets-light', testInfo);
  });

  test('evaluate: run history tab (light)', async ({ goosePage }, testInfo) => {
    await bootstrap(goosePage);

    if ((await gotoHashRouteOrWelcome(goosePage, '/evaluate')) === 'welcome') {
      test.skip(true, 'Requires a configured provider (otherwise app is on /welcome)');
    }

    if (!(await clickTabIfPresent(goosePage, /^run history$/i))) {
      test.skip(true, 'Run History tab not present');
    }

    await goosePage.waitForTimeout(150);

    const heading = await goosePage
      .getByRole('heading', { name: /^run history$/i })
      .first()
      .isVisible()
      .catch(() => false);

    if (!heading) test.skip(true, 'Run History tab not ready');

    await runContrastAudit(goosePage, 'evaluate-runs-light', testInfo);
  });

  test('extensions (light)', async ({ goosePage }, testInfo) => {
    await bootstrap(goosePage);

    if ((await gotoHashRouteOrWelcome(goosePage, '/extensions')) === 'welcome') {
      test.skip(true, 'Requires a configured provider (otherwise app is on /welcome)');
    }

    const title = await goosePage.locator('text=Extensions').isVisible().catch(() => false);
    if (!title) test.skip(true, 'Extensions view not ready');

    await runContrastAudit(goosePage, 'extensions-light', testInfo);
  });

  test.describe('dark mode', () => {
    test('initial (dark)', async ({ goosePage }, testInfo) => {
      await bootstrap(goosePage);
      await enableDarkMode(goosePage);
      await runContrastAudit(goosePage, 'initial-dark', testInfo);
    });

    test('evaluate: datasets tab (dark)', async ({ goosePage }, testInfo) => {
      await bootstrap(goosePage);

      if ((await gotoHashRouteOrWelcome(goosePage, '/evaluate')) === 'welcome') {
        test.skip(true, 'Requires a configured provider (otherwise app is on /welcome)');
      }

      await enableDarkMode(goosePage);

      if (!(await clickTabIfPresent(goosePage, /^datasets$/i))) {
        test.skip(true, 'Datasets tab not present');
      }

      await goosePage.waitForTimeout(150);

      const heading = await goosePage
        .getByRole('heading', { name: /evaluation datasets/i })
        .first()
        .isVisible()
        .catch(() => false);

      if (!heading) test.skip(true, 'Datasets tab not ready');

      await runContrastAudit(goosePage, 'evaluate-datasets-dark', testInfo);
    });
  });
});
