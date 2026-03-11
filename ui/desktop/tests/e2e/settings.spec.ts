import { test } from './fixtures';
import { expect } from '@playwright/test';
import { goToHome, openSettingsAppTab } from './helpers/test-steps';

test.describe('Settings', {tag: '@release'}, () => {
  test('dark mode toggle', async ({ goosePage }) => {
    await openSettingsAppTab(goosePage);

    const darkModeButton = goosePage.getByTestId('dark-mode-button');
    const lightModeButton = goosePage.getByTestId('light-mode-button');
    const systemModeButton = goosePage.getByTestId('system-mode-button');

    await expect(darkModeButton).toBeVisible();

    const isDarkMode = await goosePage.evaluate(() => document.documentElement.classList.contains('dark'));

    if (isDarkMode) {
      await lightModeButton.click();
      await expect(goosePage.locator('html:not(.dark)')).toBeAttached();
    } else {
      await darkModeButton.click();
      await expect(goosePage.locator('html.dark')).toBeAttached();
    }

    await systemModeButton.click();

    await lightModeButton.click();

    await goToHome(goosePage);
  });
});
