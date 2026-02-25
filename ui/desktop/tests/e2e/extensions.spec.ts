import { test, expect, waitForLoadingDone } from './fixtures.electron.packaged';

const LLM_TIMEOUT = 90000;
const PLAYWRIGHT_DEEPLINK =
  'goose://extension?cmd=npx&arg=-y&arg=@playwright/mcp@latest&id=playwright&name=Playwright&description=Modern%20web%20testing%20and%20automation';

test.describe('Goose App Extensions', () => {
  test('install playwright extension and run a google cats search', async ({ goosePage }) => {
    const mainWindow = goosePage;

    await mainWindow.getByTestId('sidebar-extensions-button').click();

    await mainWindow.evaluate((link) => {
      // ExtensionInstallModal listener expects (_event, ...args), with deeplink in args[0].
      window.electron.emit('add-extension', null, link);
    }, PLAYWRIGHT_DEEPLINK);

    const installButton = mainWindow
      .getByRole('button', { name: /^(Yes|Install Anyway)$/ })
      .first();
    await installButton.waitFor({ state: 'visible', timeout: 20000 }).catch(() => {});
    if (await installButton.isVisible().catch(() => false)) {
      await installButton.click();
    }

    await expect(mainWindow.locator('#extension-playwright')).toBeVisible();
    await expect(
      mainWindow.locator('#extension-playwright button[role="switch"][data-state="checked"]')
    ).toBeVisible();

    await mainWindow.getByTestId('sidebar-home-button').click();
    const chatInput = mainWindow.locator('[data-testid="chat-input"]:visible').first();
    await expect(chatInput).toBeVisible();

    await chatInput.fill('open a browser and search on google for cats');
    await chatInput.press('Enter');
    await waitForLoadingDone(mainWindow, LLM_TIMEOUT);

    const toolCalls = mainWindow.locator('.goose-message-tool');
    await expect(toolCalls.first()).toBeVisible();

    const toolCallsText = ((await toolCalls.allTextContents()) || []).join(' ').toLowerCase();
    expect(toolCallsText).toMatch(/playwright|browser|navigate|google|cats/);

    const latestMessage = mainWindow.locator('[data-testid="message-container"]:visible').last();
    await expect(latestMessage).toContainText(/google|cats/i);
  });
});
