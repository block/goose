import { test, expect } from './fixtures.electron.packaged';

const DEFAULT_TIMEOUT = 10000;

test.describe('Goose App', () => {
  test('history is empty before first session', async ({ goosePage }) => {
    const mainWindow = goosePage;

    const showAll = mainWindow.getByText('Show All', { exact: true }).first();
    if (!(await showAll.isVisible().catch(() => false))) {
      const chatButton = mainWindow.getByRole('button', { name: /^chat$/i }).first();
      await expect(chatButton).toBeVisible({ timeout: DEFAULT_TIMEOUT });
      await chatButton.click();
    }

    await expect(mainWindow.getByTestId('chat-sessions-list')).toBeVisible({
      timeout: DEFAULT_TIMEOUT,
    });
    await expect(mainWindow.getByTestId('chat-start-new')).toBeVisible({ timeout: DEFAULT_TIMEOUT });
    await expect(mainWindow.getByTestId('chat-show-all')).toHaveCount(0);

    const chatInput = mainWindow.getByTestId('chat-input');
    if (!(await chatInput.isVisible().catch(() => false))) {
      const sidebarHomeButton = mainWindow.getByTestId('sidebar-home-button');
      if (await sidebarHomeButton.isVisible().catch(() => false)) {
        await sidebarHomeButton.click();
      } else {
        const homeButton = mainWindow.getByRole('button', { name: /^home$/i }).first();
        await expect(homeButton).toBeVisible({ timeout: DEFAULT_TIMEOUT });
        await homeButton.click();
      }
    }
    await expect(chatInput).toBeVisible({ timeout: DEFAULT_TIMEOUT });

    const alertTrigger = mainWindow.getByTestId('bottom-menu-alert-trigger');
    await expect(alertTrigger).toBeVisible({ timeout: DEFAULT_TIMEOUT });
    await alertTrigger.click();

    const contextWindowAlert = mainWindow
      .getByTestId('alert-box')
      .filter({ has: mainWindow.getByText('Context window', { exact: true }) })
      .first();
    await expect(contextWindowAlert).toBeVisible({ timeout: DEFAULT_TIMEOUT });
    await expect(contextWindowAlert.getByTestId('alert-progress-current')).toHaveText(/^0$/);
  });
});
