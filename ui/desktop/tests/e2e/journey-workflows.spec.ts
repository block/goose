import { test, expect } from './fixtures';
import { bootstrapFirstRunUI, clickSidebarItem } from './journey-helpers';

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

  // Recipes
  await clickSidebarItem(goosePage, 'recipes');
  await goosePage.waitForURL(/#\/(recipes|welcome)/i);
  if (/\/welcome/i.test(goosePage.url())) {
    test.skip(true, 'requires a configured provider (otherwise app is on /welcome)');
  }
  await expect(goosePage.getByText(/recipes/i).first()).toBeVisible();

  // Pipelines
  await clickSidebarItem(goosePage, 'pipelines');
  await goosePage.waitForURL(/#\/pipelines/i);
  await expect(goosePage.getByText(/pipelines/i).first()).toBeVisible();

  // Scheduler
  await clickSidebarItem(goosePage, 'scheduler');
  await goosePage.waitForURL(/#\/schedules/i);
  await expect(goosePage.getByText(/schedule/i).first()).toBeVisible();
});
