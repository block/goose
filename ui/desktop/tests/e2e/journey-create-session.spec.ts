import { test, expect } from './fixtures';
import { bootstrapFirstRunUI, hashRouteUrl } from './journey-helpers';

test.skip(
  process.env.RUN_E2E_PROVIDER_JOURNEYS !== 'true',
  'requires RUN_E2E_PROVIDER_JOURNEYS=true'
);

/**
 * User journey: create a new chat session and send a message.
 *
 * Requires a configured provider. If the app redirects to /welcome, we skip.
 */

test('journey: create session (chat)', async ({ goosePage }) => {
  await bootstrapFirstRunUI(goosePage);

  await goosePage.goto(hashRouteUrl(goosePage, '/pair'));
  await goosePage.waitForURL(/#\/(pair|welcome)/i);

  if (/\/welcome/i.test(goosePage.url())) {
    test.skip(true, 'requires a configured provider (otherwise app is on /welcome)');
  }

  const input = goosePage.getByTestId('chat-input');
  await expect(input).toBeVisible();

  await input.click();
  await input.fill('hello from e2e');
  await goosePage.keyboard.press('Enter');

  await expect(goosePage.getByText('hello from e2e')).toBeVisible();
});
