import { test, expect } from './fixtures';
import { bootstrapFirstRunUI, clickSidebarItem } from './journey-helpers';

test.skip(
  process.env.RUN_E2E_PROVIDER_JOURNEYS !== 'true',
  'requires RUN_E2E_PROVIDER_JOURNEYS=true'
);

/**
 * User journey: open Settings → Models and interact with model switching UI.
 */

test('journey: settings → models', async ({ goosePage }) => {
  await bootstrapFirstRunUI(goosePage);

  await clickSidebarItem(goosePage, 'settings');
  await goosePage.waitForURL(/#\/(settings|welcome)/i);

  if (/\/welcome/i.test(goosePage.url())) {
    test.skip(true, 'requires a configured provider (otherwise app is on /welcome)');
  }

  await expect(goosePage.getByTestId('settings-models-tab')).toBeVisible();
  await expect(goosePage.getByRole('button', { name: /switch models/i })).toBeVisible();

  await goosePage.getByRole('button', { name: /switch models/i }).click();
  await expect(goosePage.getByRole('heading', { name: /switch models/i })).toBeVisible();
  await goosePage.getByRole('button', { name: /cancel/i }).click();
});
