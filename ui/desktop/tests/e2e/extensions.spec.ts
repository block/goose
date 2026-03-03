import { test } from './fixtures.electron.packaged';
import { expect } from '@playwright/test';
import { waitForLoadingDone } from './helpers/video';
import { join } from 'path';

const { runningQuotes } = require('./basic-mcp');

const LLM_TIMEOUT = 30000;
const PLAYWRIGHT_DEEPLINK =
  'goose://extension?cmd=npx&arg=-y&arg=@playwright/mcp@latest&id=playwright&name=Playwright&description=Modern%20web%20testing%20and%20automation';

test.describe('Goose App Extensions', {tag: '@release'}, () => {
  test('install playwright extension', async ({ goosePage }) => {
    await goosePage.getByTestId('sidebar-extensions-button').click();

    await goosePage.evaluate((link) => {
      // ExtensionInstallModal listener expects (_event, ...args), with deeplink in args[0].
      window.electron.emit('add-extension', null, link);
    }, PLAYWRIGHT_DEEPLINK);

    const installButton = goosePage
      .getByRole('button', { name: /^(Yes|Install Anyway)$/ })
      .first();
    await installButton.waitFor({ state: 'visible', timeout: 20000 }).catch(() => {});
    if (await installButton.isVisible().catch(() => false)) {
      await installButton.click();
    }

    await expect(goosePage.locator('#extension-playwright')).toBeVisible();
    await expect(
      goosePage.locator('#extension-playwright button[role="switch"][data-state="checked"]')
    ).toBeVisible();

    await goosePage.getByTestId('sidebar-home-button').click();
    const chatInput = goosePage.locator('[data-testid="chat-input"]:visible').first();
    await expect(chatInput).toBeVisible();

    await chatInput.fill('open a browser and search on google for cats');
    await chatInput.press('Enter');
    await waitForLoadingDone(goosePage, LLM_TIMEOUT);

    const toolCalls = goosePage.locator('.goose-message-tool');
    await expect(toolCalls.first()).toBeVisible();

    const toolCallsText = ((await toolCalls.allTextContents()) || []).join(' ').toLowerCase();
    expect(toolCallsText).toMatch(/playwright|browser|navigate|google|cats/);

    const latestMessage = goosePage.locator('[data-testid="message-container"]:visible').last();
    await expect(latestMessage).toContainText(/google|cats/i);
  });

  test('add custom extension', async ({ goosePage }) => {
    await goosePage.getByTestId('sidebar-extensions-button').click();

    await goosePage.getByRole('button', { name: 'Add custom extension' }).click();

    await goosePage.getByPlaceholder('Enter extension name...').fill('Running Quotes');
    await goosePage.getByPlaceholder('Optional description...').fill('Inspirational running quotes MCP server');
    const mcpScriptPath = join(__dirname, 'basic-mcp.ts');
    await goosePage.getByPlaceholder('e.g. npx -y @modelcontextprotocol/my-extension <filepath>').fill(`node ${mcpScriptPath}`);

    await goosePage.getByTestId('extension-submit-btn').click();

    await expect(goosePage.locator('#extension-running-quotes')).toBeVisible();
    await expect(
      goosePage.locator('#extension-running-quotes button[role="switch"][data-state="checked"]')
    ).toBeVisible();

    await goosePage.getByTestId('sidebar-home-button').click();

    const chatInput = goosePage.getByTestId('chat-input');
    await chatInput.fill('Can you give me an inspirational running quote using the runningQuote tool?');
    await chatInput.press('Enter');

    await waitForLoadingDone(goosePage, LLM_TIMEOUT);

    const lastMessage = goosePage.locator('.goose-message').last();
    const outputText = await lastMessage.textContent();

    const containsKnownQuote = runningQuotes.some(({ quote, author }) =>
      outputText?.includes(`"${quote}" - ${author}`)
    );
    expect(containsKnownQuote).toBe(true);
  });
});
