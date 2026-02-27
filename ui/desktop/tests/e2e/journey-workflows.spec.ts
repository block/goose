import { test, expect } from './fixtures';
import { assertNotOnErrorBoundary, bootstrapFirstRunUI, hashRouteUrl } from './journey-helpers';

test.skip(
  process.env.RUN_E2E_PROVIDER_JOURNEYS !== 'true',
  'requires RUN_E2E_PROVIDER_JOURNEYS=true'
);

/**
 * User journey: workflows navigation (recipes / pipelines / schedules).
 *
 * Requires a configured provider (guarded route). If the app redirects to /welcome, we skip.
 */

test('journey: workflows navigation', async ({ goosePage }) => {
  await bootstrapFirstRunUI(goosePage);
  await assertNotOnErrorBoundary(goosePage, 'workflows: after bootstrap');

  // Recipes
  await goosePage.goto(hashRouteUrl(goosePage, '/recipes'));
  await goosePage.waitForURL(/#\/(recipes|welcome)/i);
  if (/\/welcome/i.test(goosePage.url())) {
    test.skip(true, 'requires a configured provider (otherwise app is on /welcome)');
  }
  await assertNotOnErrorBoundary(goosePage, 'workflows: recipes');
  await expect(goosePage.getByText(/recipes/i).first()).toBeVisible();

  // Pipelines
  await goosePage.goto(hashRouteUrl(goosePage, '/pipelines'));
  await goosePage.waitForURL(/#\/pipelines/i);
  await assertNotOnErrorBoundary(goosePage, 'workflows: pipelines');
  await expect(goosePage.getByText(/pipelines/i).first()).toBeVisible();

  // Scheduler
  await goosePage.goto(hashRouteUrl(goosePage, '/schedules'));
  await goosePage.waitForURL(/#\/schedules/i);
  await assertNotOnErrorBoundary(goosePage, 'workflows: schedules');
  await expect(goosePage.getByText(/schedule/i).first()).toBeVisible();
});
