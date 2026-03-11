import { test } from './fixtures';
import { expect } from '@playwright/test';
import { createCustomExtension, expectExtensionIsEnabled, expectLastChatMessageContains, getLastAssistantMessageText, getToolCalls, goToExtensions, goToHome, sendMessage } from './helpers/test-steps';
import { join } from 'path';

const { runningQuotes } = require('./basic-mcp');
const PLAYWRIGHT_DEEPLINK =
  'goose://extension?cmd=npx&arg=-y&arg=@playwright/mcp@latest&id=playwright&name=Playwright&description=Modern%20web%20testing%20and%20automation';

test.describe('Goose App Extensions', {tag: '@release'}, () => {
  test('install playwright extension', async ({ goosePage }) => {
    test.setTimeout(120000);
    await goToExtensions(goosePage);

    await goosePage.evaluate((link) => {
      // ExtensionInstallModal listener expects (_event, ...args), with deeplink in args[0].
      window.electron.emit('add-extension', null, link);
    }, PLAYWRIGHT_DEEPLINK);

    const installButton = goosePage
      .getByRole('button', { name: /^(Yes|Install Anyway)$/ })
      .first();
    await installButton.click();

    await expectExtensionIsEnabled(goosePage, 'playwright');

    await goToHome(goosePage);

    await sendMessage(goosePage, 'open a browser and search on google for cats');

    const toolCalls = getToolCalls(goosePage);
    await expect(toolCalls.first()).toBeVisible();

    const toolCallsText = ((await toolCalls.allTextContents()) || []).join(' ').toLowerCase();
    expect(toolCallsText).toMatch(/playwright|browser|navigate|google|cats/);

    await expectLastChatMessageContains(goosePage, /google|cats/i);
  });

  test('add custom extension', async ({ goosePage }) => {
    await goToExtensions(goosePage);

    const mcpScriptPath = join(__dirname, 'basic-mcp.ts');
    await createCustomExtension(goosePage, {
      name: 'Running Quotes',
      description: 'Inspirational running quotes MCP server',
      command: `node ${mcpScriptPath}`,
    });

    await expectExtensionIsEnabled(goosePage, 'running-quotes');

    await goToHome(goosePage);
    await sendMessage(goosePage, 'Can you give me an inspirational running quote using the runningQuote tool?');

    const outputText = await getLastAssistantMessageText(goosePage);

    const containsKnownQuote = runningQuotes.some(({ quote, author }) =>
      outputText.includes(`"${quote}" - ${author}`)
    );
    expect(containsKnownQuote).toBe(true);
  });
});
