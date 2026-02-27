import { test, expect } from './fixtures';
import { bootstrapFirstRunUI, hashRouteUrl } from './journey-helpers';

/**
 * User journey: configure providers.
 *
 * This route is NOT guarded by ProviderGuard, so it should be reachable even when
 * no provider is configured yet.
 */

test('journey: configure providers', async ({ goosePage }) => {
  await goosePage.goto(hashRouteUrl(goosePage, '/configure-providers'));
  await goosePage.waitForURL(/#\/configure-providers/i);

  await bootstrapFirstRunUI(goosePage);

  await expect(goosePage.getByText(/provider/i).first()).toBeVisible();
});
