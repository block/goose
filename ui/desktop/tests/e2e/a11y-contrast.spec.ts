import axe from 'axe-core';
import type { Page, TestInfo } from '@playwright/test';
import { test, expect } from './fixtures';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname } from 'node:path';

test.skip(
  process.env.RUN_A11Y_CONTRAST !== 'true',
  'Set RUN_A11Y_CONTRAST=true to enable this e2e contrast audit'
);

// This suite can be slow on first run (starting goosed, reading user config, etc.).
test.describe.configure({ timeout: 180_000 });

type AxeRunResult = {
  violations: Array<{ id: string; description: string; help: string; impact: string | null; nodes: unknown[] }>;
};

async function ensureAxeInjected(page: Page) {
  const hasAxe = await page.evaluate(() => Boolean((window as unknown as { axe?: unknown }).axe)).catch(() => false);
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

  // Helpful when running from CLI and not opening the HTML report.
  // (The detailed nodes/targets remain in the JSON.)
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

test('a11y: color contrast (axe-core)', async ({ goosePage }, testInfo) => {
  await goosePage.waitForLoadState('domcontentloaded');

  await goosePage.waitForFunction(
    () => {
      const root = document.getElementById('root');
      return root && root.children.length > 0;
    },
    undefined,
    { timeout: 30_000 }
  );

  await dismissOptionalChooseModelModal(goosePage);
  await dismissOptionalTelemetryModal(goosePage);

  // Always audit whatever screen we land on (welcome, chat, etc.).
  await runContrastAudit(goosePage, 'initial-light', testInfo);

  // Optional navigation pass. If we can't navigate due to first-run state,
  // we still have the baseline audit above.
  if (await navigateSidebar(goosePage, 'home')) {
    if (await goosePage.locator('[data-testid="chat-input"]').isVisible().catch(() => false)) {
      await runContrastAudit(goosePage, 'chat-light', testInfo);
    }
  }

  if (await navigateSidebar(goosePage, 'settings')) {
    if (await goosePage.locator('[data-testid="settings-app-tab"]').isVisible().catch(() => false)) {
      await runContrastAudit(goosePage, 'settings-light', testInfo);
    }
  }

  if (await navigateSidebar(goosePage, 'monitoring')) {
    if (await goosePage.locator('text=Monitoring').isVisible().catch(() => false)) {
      await runContrastAudit(goosePage, 'monitoring-light', testInfo);
    }
  }

  if (await navigateSidebar(goosePage, 'evaluate')) {
    if (await goosePage.locator('text=Evaluate').isVisible().catch(() => false)) {
      await runContrastAudit(goosePage, 'evaluate-light', testInfo);
    }
  }

  if (await navigateSidebar(goosePage, 'extensions')) {
    if (await goosePage.locator('text=Extensions').isVisible().catch(() => false)) {
      await runContrastAudit(goosePage, 'extensions-light', testInfo);
    }
  }

  // Dark-mode pass: toggle via the class to exercise the dark token palette.
  await goosePage.evaluate(() => {
    document.documentElement.classList.add('dark');
  });
  await goosePage.waitForTimeout(250);

  await runContrastAudit(goosePage, 'initial-dark', testInfo);

  if (await navigateSidebar(goosePage, 'home')) {
    if (await goosePage.locator('[data-testid="chat-input"]').isVisible().catch(() => false)) {
      await runContrastAudit(goosePage, 'chat-dark', testInfo);
    }
  }

  if (await navigateSidebar(goosePage, 'settings')) {
    if (await goosePage.locator('[data-testid="settings-app-tab"]').isVisible().catch(() => false)) {
      await runContrastAudit(goosePage, 'settings-dark', testInfo);
    }
  }
});
