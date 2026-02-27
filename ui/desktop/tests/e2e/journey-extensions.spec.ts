import { test, expect } from './fixtures';
import { assertNotOnErrorBoundary, bootstrapFirstRunUI, hashRouteUrl } from './journey-helpers';

test.skip(
  process.env.RUN_E2E_PROVIDER_JOURNEYS !== 'true',
  'requires RUN_E2E_PROVIDER_JOURNEYS=true'
);

/**
 * User journey: extensions navigation.
 *
 * Requires a configured provider (guarded route). If the app redirects to /welcome, we skip.
 */

test('journey: extensions navigation', async ({ goosePage }) => {
  await bootstrapFirstRunUI(goosePage);
  await assertNotOnErrorBoundary(goosePage, 'extensions: after bootstrap');

  await goosePage.goto(hashRouteUrl(goosePage, '/extensions'));
  await goosePage.waitForURL(/#\/(extensions|welcome)/i);

  if (/\/welcome/i.test(goosePage.url())) {
    test.skip(true, 'requires a configured provider (otherwise app is on /welcome)');
  }

  await assertNotOnErrorBoundary(goosePage, 'extensions: after navigation');
  await expect(goosePage.getByText(/extensions/i).first()).toBeVisible();
});
