import { test, expect } from './fixtures';
import { bootstrapFirstRunUI, clickSidebarItem } from './journey-helpers';

test.skip(
  process.env.RUN_E2E_PROVIDER_JOURNEYS !== 'true',
  'requires RUN_E2E_PROVIDER_JOURNEYS=true'
);

/**
 * User journey: Evaluate → Datasets
 *
 * Creates a dataset via the UI and verifies it appears in the list.
 */

test('journey: evaluate → datasets (create dataset)', async ({ goosePage }) => {
  await bootstrapFirstRunUI(goosePage);

  await clickSidebarItem(goosePage, 'evaluate');
  await goosePage.waitForURL(/#\/evaluate/i);

  await expect(goosePage.getByText('Evaluate').first()).toBeVisible();

  await goosePage.getByRole('button', { name: /^datasets$/i }).click();
  await expect(goosePage.getByRole('heading', { name: /evaluation datasets/i })).toBeVisible();

  await goosePage.getByRole('button', { name: /new dataset/i }).click();

  const datasetName = `e2e dataset ${Date.now()}`;

  await goosePage.locator('#dataset-name').fill(datasetName);
  await goosePage.locator('#dataset-description').fill('created by playwright journey');

  // Fill the first test case row (inputs are placeholder-based).
  await goosePage.getByPlaceholder('User message...').fill('route this to the right agent');
  await goosePage.getByPlaceholder('agent').fill('agent');
  await goosePage.getByPlaceholder('mode').fill('default');
  await goosePage.getByPlaceholder('tag1, tag2').fill('e2e');

  await goosePage.getByRole('button', { name: /create dataset/i }).click();

  // Back to list; newly created dataset name should appear.
  await expect(goosePage.getByText(datasetName)).toBeVisible();
});
