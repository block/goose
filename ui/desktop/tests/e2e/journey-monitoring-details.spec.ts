import { test, expect } from './fixtures';
import { bootstrapFirstRunUI, clickSidebarItem } from './journey-helpers';

test.skip(
  process.env.RUN_E2E_PROVIDER_JOURNEYS !== 'true',
  'requires RUN_E2E_PROVIDER_JOURNEYS=true'
);

/**
 * User journey: Monitoring → details
 *
 * Navigates to Monitoring, switches to Live and Tool Analytics tabs,
 * and asserts key UI elements exist.
 */

test('journey: monitoring → live + tool analytics', async ({ goosePage }) => {
  await bootstrapFirstRunUI(goosePage);

  await clickSidebarItem(goosePage, 'monitoring');
  await goosePage.waitForURL(/#\/monitoring/i);

  await expect(goosePage.getByText('Monitoring').first()).toBeVisible();

  // Live tab
  await goosePage.getByRole('button', { name: /^live$/i }).click();

  // Accept either loaded state or error state.
  const liveHeading = goosePage.getByRole('heading', { name: /live monitoring/i }).first();
  const liveRetry = goosePage.getByRole('button', { name: /retry/i }).first();
  await expect(liveHeading.or(liveRetry)).toBeVisible();

  // Tool Analytics tab
  await goosePage.getByRole('button', { name: /tool analytics/i }).click();
  await expect(goosePage.getByText(/tool usage|daily tool activity|failed to load/i).first()).toBeVisible();
});
