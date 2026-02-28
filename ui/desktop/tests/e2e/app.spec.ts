import { test as base, expect, waitForLoadingDone } from './fixtures.electron.packaged';
import { Page } from '@playwright/test';
import { showTestName, clearTestName } from './test-overlay';
import { join } from 'path';

const { runningQuotes } = require('./basic-mcp');

const DEFAULT_TIMEOUT = 10000;
const LLM_TIMEOUT = 30000;

// Define provider interface
type Provider = {
  name: string;
};

// Create test fixture type
type TestFixtures = {
  provider: Provider;
};

// Define available providers, keeping as a list of objects for easy expansion
const providers: Provider[] = [
  { name: 'Databricks' }
];

// Create test with fixtures
const test = base.extend<TestFixtures>({
  provider: [providers[0], { option: true }], // Default to first provider (Databricks)
});

let mainWindow: Page;

test.beforeEach(async ({ goosePage }, testInfo) => {
  mainWindow = goosePage;

  const testName = testInfo.titlePath[testInfo.titlePath.length - 1];

  const providerSuite = testInfo.titlePath.find(t => t.startsWith('Provider:'));
  const providerName = providerSuite ? providerSuite.split(': ')[1] : undefined;

  if (!process.env.NO_TEST_OVERLAY) {
    console.log(`Setting overlay for test: "${testName}"${providerName ? ` (Provider: ${providerName})` : ''}`);
    await showTestName(mainWindow, testName, providerName);
  }
});

test.afterEach(async () => {
  if (mainWindow && !process.env.NO_TEST_OVERLAY) {
    await clearTestName(mainWindow);
  }
});

// Helper function to select a provider
async function selectProvider(mainWindow: Page, provider: Provider) {
  console.log(`Selecting provider: ${provider.name}`);

  // Each test gets a fresh app with an isolated config (via GOOSE_PATH_ROOT in fixtures).
  // The config is seeded with GOOSE_PROVIDER, so the chat interface should be available.
  const chatInput = mainWindow.getByTestId('chat-input');
  try {
    await expect(chatInput).toBeVisible({ timeout: DEFAULT_TIMEOUT });
    console.log('Provider already configured, chat interface is available');
    return;
  } catch {
    // Not on chat screen yet, continue with provider setup
  }

  // Check if we're on the welcome screen with "Other Providers" section
  const otherProviders = mainWindow.getByText('Other Providers');
  if (await otherProviders.isVisible({ timeout: 3000 }).catch(() => false)) {
    console.log('Found "Other Providers" section, clicking "Go to Provider Settings" link...');
    await mainWindow.getByRole('button', { name: 'Go to Provider Settings' }).click();
    console.log('Navigated to Provider Settings');
  }

  // Now we should be on the "Other providers" page with provider cards
  console.log(`Looking for ${provider.name} provider card...`);

  const providerCardTestId = `provider-card-${provider.name.toLowerCase()}`;
  const launchButton = mainWindow.getByTestId(providerCardTestId).getByRole('button', { name: 'Launch' });

  await expect(launchButton).toBeVisible({ timeout: 5000 });
  console.log(`Found Launch button in ${provider.name} card, clicking it...`);
  await launchButton.click();

  // Wait for "Choose Model" dialog to appear and select a model
  console.log('Waiting for model selection dialog...');
  const chooseModelHeading = mainWindow.getByText('Choose Model');
  if (await chooseModelHeading.isVisible({ timeout: 5000 }).catch(() => false)) {
    console.log('Model selection dialog appeared, waiting for models to load...');
    // The "Select model" button starts enabled and only disables during loading (UI bug)
    await mainWindow.waitForTimeout(5000);
    console.log('Waited for models to load');

    await mainWindow.getByRole('button', { name: 'Select model' }).click();
    console.log('Clicked "Select model" button');
  }

  // Navigate to home/chat after provider configuration
  console.log('Navigating to home/chat...');
  const homeButton = mainWindow.getByTestId('sidebar-home-button');
  if (await homeButton.isVisible().catch(() => false)) {
    await homeButton.click();
  }

  // Wait for chat interface to appear
  await expect(mainWindow.getByTestId('chat-input')).toBeVisible({ timeout: DEFAULT_TIMEOUT });

}

test.describe('Goose App', () => {

  test.describe('General UI', () => {
    test('dark mode toggle', async () => {
      console.log('Testing dark mode toggle...');

      await expect(mainWindow.getByTestId('chat-input')).toBeVisible({ timeout: DEFAULT_TIMEOUT });

      // Navigate to Settings via sidebar
      await mainWindow.getByTestId('sidebar-settings-button').click();

      // Navigate to App tab
      await mainWindow.getByTestId('settings-app-tab').click();

      const darkModeButton = mainWindow.getByTestId('dark-mode-button');
      const lightModeButton = mainWindow.getByTestId('light-mode-button');
      const systemModeButton = mainWindow.getByTestId('system-mode-button');

      await expect(darkModeButton).toBeVisible();

      // Get initial state
      const isDarkMode = await mainWindow.evaluate(() => document.documentElement.classList.contains('dark'));
      console.log('Initial dark mode state:', isDarkMode);

      if (isDarkMode) {
        await lightModeButton.click();
        await expect(mainWindow.locator('html:not(.dark)')).toBeAttached();
      } else {
        await darkModeButton.click();
        await expect(mainWindow.locator('html.dark')).toBeAttached();
      }

      // Check that system mode is clickable
      await systemModeButton.click();

      // Toggle back to light mode
      await lightModeButton.click();

      // Navigate back to home
      await mainWindow.getByTestId('sidebar-home-button').click();
    });
  });

  for (const provider of providers) {
    test.describe(`Provider: ${provider.name}`, () => {
      test.beforeEach(async () => {
        await selectProvider(mainWindow, provider);
      });

      test.describe('Chat', () => {
        test('chat interaction', async () => {
          console.log(`Testing chat interaction with ${provider.name}...`);

          const chatInput = mainWindow.getByTestId('chat-input');
          await expect(chatInput).toBeVisible();

          await chatInput.fill('Hello, can you help me with a simple task?');
          await chatInput.press('Enter');

          // Wait for response to complete
          console.log('Waiting for response...');
          await waitForLoadingDone(mainWindow, LLM_TIMEOUT);
          console.log('Response complete');

          // Verify response
          const response = mainWindow.getByTestId('message-container').last();
          await expect(response).toBeVisible();
          const responseText = await response.textContent();
          expect(responseText).toBeTruthy();
          expect(responseText?.length).toBeGreaterThan(0);
        });

        test('verify chat history', async () => {
          console.log(`Testing chat history with ${provider.name}...`);

          const chatInput = mainWindow.getByTestId('chat-input');
          await chatInput.fill('What is 2+2?');
          await chatInput.press('Enter');

          // Wait for response to complete
          await waitForLoadingDone(mainWindow, LLM_TIMEOUT);

          // Verify response
          const response = mainWindow.getByTestId('message-container').last();
          const responseText = await response.textContent();
          expect(responseText).toBeTruthy();

          // Check for message history
          await expect(mainWindow.getByTestId('message-container')).toHaveCount(2, { timeout: 5000 });

          // Test command history (up arrow)
          const chatInputForHistory = mainWindow.getByTestId('chat-input');
          await chatInputForHistory.press('Control+ArrowUp');
          await expect(chatInputForHistory).toHaveValue('What is 2+2?');
        });
      });

      test('MCP integration - add extension and use tool', async () => {
          // Navigate to Extensions via sidebar
          await mainWindow.getByTestId('sidebar-extensions-button').click();

          // Add custom extension
          await mainWindow.getByRole('button', { name: 'Add custom extension' }).click();

          // Fill the extension form
          await mainWindow.getByPlaceholder('Enter extension name...').fill('Running Quotes');
          await mainWindow.getByPlaceholder('Optional description...').fill('Inspirational running quotes MCP server');
          const mcpScriptPath = join(__dirname, 'basic-mcp.ts');
          await mainWindow.getByPlaceholder('e.g. npx -y @modelcontextprotocol/my-extension <filepath>').fill(`node ${mcpScriptPath}`);

          // Submit
          await mainWindow.getByTestId('extension-submit-btn').click();

          // Wait for the extension to appear and be enabled
          await mainWindow.locator('#extension-running-quotes').waitFor({ timeout: DEFAULT_TIMEOUT });
          await expect(
            mainWindow.locator('#extension-running-quotes button[role="switch"][data-state="checked"]')
          ).toBeVisible({ timeout: DEFAULT_TIMEOUT });

          // Navigate back to home
          await mainWindow.getByTestId('sidebar-home-button').click();

          // Send a message requesting a running quote
          const chatInput = mainWindow.getByTestId('chat-input');
          await chatInput.fill('Can you give me an inspirational running quote using the runningQuote tool?');
          await chatInput.press('Enter');

          // Wait for goose to finish responding
          await waitForLoadingDone(mainWindow, LLM_TIMEOUT);

          // Verify the response contains a known quote
          const lastMessage = mainWindow.locator('.goose-message').last();
          const outputText = await lastMessage.textContent();

          const containsKnownQuote = runningQuotes.some(({ quote, author }) =>
            outputText?.includes(`"${quote}" - ${author}`)
          );
          expect(containsKnownQuote).toBe(true);
      });
    });
  }
});
