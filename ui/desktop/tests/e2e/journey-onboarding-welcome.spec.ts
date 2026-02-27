import { test, expect } from './fixtures';
import { bootstrapFirstRunUI, hashRouteUrl } from './journey-helpers';

/**
 * User journey: first-run onboarding.
 *
 * This should work in any environment because it does not require a configured provider.
 */

test('journey: onboarding (welcome)', async ({ goosePage }) => {
  await goosePage.goto(hashRouteUrl(goosePage, '/welcome'));
  await goosePage.waitForURL(/#\/welcome/i);

  await bootstrapFirstRunUI(goosePage);

  await expect(goosePage.getByRole('heading', { name: /welcome to goose/i })).toBeVisible();
  await expect(goosePage.getByText(/choose a model provider/i)).toBeVisible();

  await expect(goosePage.getByRole('button', { name: /get started/i })).toBeVisible();
});
