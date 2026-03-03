import { test } from './fixtures.electron.packaged';
import { expect } from '@playwright/test';

test.describe('Settings', {tag: '@release'}, () => {
  test('dark mode toggle', async ({ goosePage }) => {
    console.log('Testing dark mode toggle...');

    await expect(goosePage.getByTestId('chat-input')).toBeVisible();

    await goosePage.getByTestId('sidebar-settings-button').click();

    await goosePage.getByTestId('settings-app-tab').click();

    const darkModeButton = goosePage.getByTestId('dark-mode-button');
    const lightModeButton = goosePage.getByTestId('light-mode-button');
    const systemModeButton = goosePage.getByTestId('system-mode-button');

    await expect(darkModeButton).toBeVisible();

    const isDarkMode = await goosePage.evaluate(() => document.documentElement.classList.contains('dark'));
    console.log('Initial dark mode state:', isDarkMode);

    if (isDarkMode) {
      await lightModeButton.click();
      await expect(goosePage.locator('html:not(.dark)')).toBeAttached();
    } else {
      await darkModeButton.click();
      await expect(goosePage.locator('html.dark')).toBeAttached();
    }

    await systemModeButton.click();

    await lightModeButton.click();

    await goosePage.getByTestId('sidebar-home-button').click();
  });
});
