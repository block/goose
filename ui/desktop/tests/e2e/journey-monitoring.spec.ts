import { test, expect } from './fixtures';
import { bootstrapFirstRunUI, clickSidebarItem } from './journey-helpers';

test.skip(
  process.env.RUN_E2E_PROVIDER_JOURNEYS !== 'true',
  'requires RUN_E2E_PROVIDER_JOURNEYS=true'
);

/**
 * User journey: monitoring navigation.
 *
 * Requires a configured provider (guarded route). If the app redirects to /welcome, we skip.
 */

test('journey: monitoring navigation', async ({ goosePage }) => {
  await bootstrapFirstRunUI(goosePage);

  await clickSidebarItem(goosePage, 'monitoring');
  await goosePage.waitForURL(/#\/(monitoring|welcome)/i);

  if (/\/welcome/i.test(goosePage.url())) {
    test.skip(true, 'requires a configured provider (otherwise app is on /welcome)');
  }

  await expect(goosePage.getByText(/monitoring/i).first()).toBeVisible();
});
