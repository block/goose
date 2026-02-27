import { test, expect } from './fixtures';
import { assertNotOnErrorBoundary, bootstrapFirstRunUI, hashRouteUrl } from './journey-helpers';

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
  await assertNotOnErrorBoundary(goosePage, 'create-session: after bootstrap');

  await goosePage.goto(hashRouteUrl(goosePage, '/pair'));
  await goosePage.waitForURL(/#\/(pair|welcome)/i);

  if (/\/welcome/i.test(goosePage.url())) {
    test.skip(true, 'requires a configured provider (otherwise app is on /welcome)');
  }

  await assertNotOnErrorBoundary(goosePage, 'create-session: on pair');

  const input = goosePage.getByTestId('chat-input');
  await expect(input).toBeVisible();

  await input.click();
  await input.fill('hello from e2e');
  await goosePage.keyboard.press('Enter');

  const messageVisible = goosePage.getByText('hello from e2e').first();
  const honk = goosePage.getByRole('heading', { name: /^honk!$/i });

  // Wait for either the message to appear OR an ErrorBoundary crash.
  await Promise.race([
    messageVisible.waitFor({ state: 'visible', timeout: 30_000 }),
    honk.waitFor({ state: 'visible', timeout: 30_000 }).then(() => {
      throw new Error('App crashed after sending message (ErrorBoundary "Honk!" visible)');
    }),
  ]);

  await assertNotOnErrorBoundary(goosePage, 'create-session: after send');
});
