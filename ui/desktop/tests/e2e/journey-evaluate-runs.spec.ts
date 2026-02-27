import { test, expect } from './fixtures';
import { bootstrapFirstRunUI, clickSidebarItem } from './journey-helpers';

test.skip(
  process.env.RUN_E2E_PROVIDER_JOURNEYS !== 'true',
  'requires RUN_E2E_PROVIDER_JOURNEYS=true'
);

/**
 * User journey: Evaluate → Run History
 *
 * Notes:
 * - Running evals can be slow/flaky depending on provider configuration.
 * - This test only performs an actual run when RUN_E2E_EVAL_RUNS=true.
 */

test('journey: evaluate → run history', async ({ goosePage }) => {
  await bootstrapFirstRunUI(goosePage);

  await clickSidebarItem(goosePage, 'evaluate');
  await goosePage.waitForURL(/#\/evaluate/i);

  await goosePage.getByRole('button', { name: /run history/i }).click();
  await expect(goosePage.getByRole('heading', { name: /^run history$/i })).toBeVisible();

  // If we already have runs, opening details is a stable check.
  const detailsLink = goosePage.getByRole('button', { name: /details/i }).first();
  if ((await detailsLink.count()) > 0) {
    await detailsLink.click();
    await expect(goosePage.getByRole('button', { name: /back to runs/i })).toBeVisible();
    return;
  }

  test.skip(
    process.env.RUN_E2E_EVAL_RUNS !== 'true',
    'no runs available; set RUN_E2E_EVAL_RUNS=true to allow creating an eval run'
  );

  // Attempt to run an eval using the first available dataset.
  // If no dataset exists, this will remain disabled and the test will fail with a clear assertion.
  const runButton = goosePage.getByRole('button', { name: /run eval/i });
  await expect(runButton).toBeEnabled();

  await runButton.click();

  // Wait for at least one run row to appear.
  await expect(goosePage.getByRole('button', { name: /details/i }).first()).toBeVisible({
    timeout: 120_000,
  });
});
