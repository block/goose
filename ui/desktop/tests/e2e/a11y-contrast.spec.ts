import { test, expect } from './fixtures';
import AxeBuilder from '@axe-core/playwright';

test.skip(
  process.env.RUN_A11Y_CONTRAST !== 'true',
  'Set RUN_A11Y_CONTRAST=true to enable this e2e contrast audit'
);

test('a11y: color contrast (axe-core)', async ({ goosePage }, testInfo) => {
  // The fixture waits for the app root to be present. We only need a stable layout.
  await goosePage.waitForLoadState('domcontentloaded');

  const results = await new AxeBuilder({ page: goosePage })
    // Keep this narrowly scoped to palette/contrast work.
    .withRules(['color-contrast'])
    .analyze();

  await testInfo.attach('axe-results.json', {
    body: JSON.stringify(results, null, 2),
    contentType: 'application/json',
  });

  expect(results.violations, JSON.stringify(results.violations, null, 2)).toEqual([]);
});
