import { test as base, expect } from './fixtures';
import { Page } from '@playwright/test';
import { showTestName, clearTestName } from './test-overlay';
import { join } from 'path';

const { runningQuotes } = require('./basic-mcp');

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

  console.log(`Setting overlay for test: "${testName}"${providerName ? ` (Provider: ${providerName})` : ''}`);
  await showTestName(mainWindow, testName, providerName);
});

test.afterEach(async () => {
  if (mainWindow) {
    await clearTestName(mainWindow);
  }
});

// Helper function to select a provider
async function selectProvider(mainWindow: any, provider: Provider) {
  console.log(`Selecting provider: ${provider.name}`);

  // Each test gets a fresh app with an isolated config (via GOOSE_PATH_ROOT in fixtures).
  // The config is seeded with GOOSE_PROVIDER, so the chat interface should be available.
  const chatInput = await mainWindow.waitForSelector('[data-testid="chat-input"]', {
    timeout: 10000,
    state: 'visible'
  }).catch(() => null);

  if (chatInput) {
    console.log('Provider already configured, chat interface is available');
    return;
  }

  // Check if we're on the welcome screen with "Other Providers" section
  const otherProvidersSection = await mainWindow.waitForSelector('text="Other Providers"', {
    timeout: 3000,
    state: 'visible'
  }).catch(() => null);

  if (otherProvidersSection) {
    console.log('Found "Other Providers" section, clicking "Go to Provider Settings" link...');
    // Click the "Go to Provider Settings" link (includes arrow →)
    const providerSettingsLink = await mainWindow.waitForSelector('button:has-text("Go to Provider Settings")', {
      timeout: 3000,
      state: 'visible'
    });
    await providerSettingsLink.click();
    await mainWindow.waitForTimeout(1000);

    // We should now be in Settings -> Models tab
    console.log('Navigated to Provider Settings');
  }

  // Now we should be on the "Other providers" page with provider cards
  console.log(`Looking for ${provider.name} provider card...`);

  // Wait for the provider cards to load
  await mainWindow.waitForTimeout(1000);

  // Find the Launch button within the specific provider card using its data-testid
  console.log(`Looking for ${provider.name} card with Launch button...`);

  try {
    // Each provider card has data-testid="provider-card-{provider-name-lowercase}"
    const providerCardTestId = `provider-card-${provider.name.toLowerCase()}`;
    const launchButton = mainWindow.locator(`[data-testid="${providerCardTestId}"] button:has-text("Launch")`);

    await launchButton.waitFor({ state: 'visible', timeout: 5000 });
    console.log(`Found Launch button in ${provider.name} card, clicking it...`);
    await launchButton.click();
    await mainWindow.waitForTimeout(1000);

    // Wait for "Choose Model" dialog to appear and select a model
    console.log('Waiting for model selection dialog...');
    const chooseModelDialog = await mainWindow.waitForSelector('text="Choose Model"', {
      timeout: 5000,
      state: 'visible'
    }).catch(() => null);

    if (chooseModelDialog) {
      console.log('Model selection dialog appeared, waiting for models to load...');

      // The "Select model" button starts enabled and only disables during loading (UI bug)
      // So we wait for a fixed timeout to ensure models are loaded
      await mainWindow.waitForTimeout(5000);
      console.log('Waited for models to load');

      const confirmButton = await mainWindow.waitForSelector('button:has-text("Select model")', {
        timeout: 5000,
        state: 'visible'
      });

      console.log('Clicking "Select model" button');
      await confirmButton.click();
      await mainWindow.waitForTimeout(2000);
    }
  } catch (error) {
    console.error(`Failed to find or click Launch button in ${provider.name} card:`, error);
    throw error;
  }

  // Navigate to home/chat after provider configuration
  console.log('Navigating to home/chat...');
  const homeButton = await mainWindow.waitForSelector('[data-testid="sidebar-home-button"]', {
    timeout: 5000
  }).catch(() => null);

  if (homeButton) {
    await homeButton.click();
    await mainWindow.waitForTimeout(1000);
  }

  // Wait for chat interface to appear
  const chatTextareaAfterConfig = await mainWindow.waitForSelector('[data-testid="chat-input"]',
    { timeout: 10000 });
  expect(await chatTextareaAfterConfig.isVisible()).toBe(true);

  // Take screenshot of chat interface
  await mainWindow.screenshot({ path: `test-results/chat-interface-${provider.name.toLowerCase()}.png` });
}

test.describe('Goose App', () => {
  // No need for beforeAll/afterAll - the fixture handles app launch and cleanup!

  test.describe('General UI', () => {
    test('dark mode toggle', async () => {
      console.log('Testing dark mode toggle...');

      // Assume the app is already configured and wait for chat input
      await mainWindow.waitForSelector('[data-testid="chat-input"]', {
        timeout: 10000
      });

      // Navigate to Settings via sidebar
      const settingsButton = await mainWindow.waitForSelector('[data-testid="sidebar-settings-button"]', {
        timeout: 5000,
        state: 'visible'
      });
      await settingsButton.click();

      // Wait for settings page to load and navigate to App tab
      await mainWindow.waitForSelector('[data-testid="settings-app-tab"]', {
        timeout: 5000,
        state: 'visible'
      });

      const appTab = await mainWindow.waitForSelector('[data-testid="settings-app-tab"]');
      await appTab.click();

      // Wait for the theme selector to be visible
      await mainWindow.waitForTimeout(1000);

      // Find and click the dark mode toggle button
      const darkModeButton = await mainWindow.waitForSelector('[data-testid="dark-mode-button"]');
      const lightModeButton = await mainWindow.waitForSelector('[data-testid="light-mode-button"]');
      const systemModeButton = await mainWindow.waitForSelector('[data-testid="system-mode-button"]');

      // Get initial state
      const isDarkMode = await mainWindow.evaluate(() => document.documentElement.classList.contains('dark'));
      console.log('Initial dark mode state:', isDarkMode);

      if (isDarkMode) {
        // Click to toggle to light mode
        await lightModeButton.click();
        await mainWindow.waitForTimeout(1000);
        const newDarkMode = await mainWindow.evaluate(() => document.documentElement.classList.contains('dark'));
        expect(newDarkMode).toBe(!isDarkMode);
        // Take screenshot to verify and pause to show the change
        await mainWindow.screenshot({ path: 'test-results/dark-mode-toggle.png' });
      } else {
        // Click to toggle to dark mode
        await darkModeButton.click();
        await mainWindow.waitForTimeout(1000);
        const newDarkMode = await mainWindow.evaluate(() => document.documentElement.classList.contains('dark'));
        expect(newDarkMode).toBe(!isDarkMode);
      }

      // check that system mode is clickable
      await systemModeButton.click();

      // Toggle back to light mode
      await lightModeButton.click();

      // Pause to show return to original state
      await mainWindow.waitForTimeout(2000);

      // Navigate back to home
      const homeButton = await mainWindow.waitForSelector('[data-testid="sidebar-home-button"]');
      await homeButton.click();
    });
  });

  for (const provider of providers) {
    test.describe(`Provider: ${provider.name}`, () => {
      test.beforeEach(async () => {
        // Select the provider before each test for this provider
        await selectProvider(mainWindow, provider);
      });

      test.describe('Chat', () => {
        test('chat interaction', async () => {
          console.log(`Testing chat interaction with ${provider.name}...`);

          // Find the chat input
          const chatInput = await mainWindow.waitForSelector('[data-testid="chat-input"]');
          expect(await chatInput.isVisible()).toBe(true);

          // Type a message
          await chatInput.fill('Hello, can you help me with a simple task?');

          // Take screenshot before sending
          await mainWindow.screenshot({ path: `test-results/${provider.name.toLowerCase()}-before-send.png` });

          // Send message
          await chatInput.press('Enter');

          // Wait for loading indicator to appear and then disappear
          console.log('Waiting for response...');
          await mainWindow.waitForSelector('[data-testid="loading-indicator"]', {
            state: 'visible',
            timeout: 5000
          });
          console.log('Loading indicator appeared');

          await mainWindow.waitForSelector('[data-testid="loading-indicator"]', {
            state: 'hidden',
            timeout: 30000
          });
          console.log('Loading indicator disappeared');

          // Get the latest response
          const response = await mainWindow.locator('[data-testid="message-container"]').last();
          expect(await response.isVisible()).toBe(true);

          // Verify response has content
          const responseText = await response.textContent();
          expect(responseText).toBeTruthy();
          expect(responseText.length).toBeGreaterThan(0);

          // Take screenshot of response
          await mainWindow.screenshot({ path: `test-results/${provider.name.toLowerCase()}-chat-response.png` });
        });

        test('verify chat history', async () => {
          console.log(`Testing chat history with ${provider.name}...`);

          // Find the chat input again
          const chatInput = await mainWindow.waitForSelector('[data-testid="chat-input"]');

          // Test message sending with a specific question
          await chatInput.fill('What is 2+2?');

          // Send message
          await chatInput.press('Enter');

          // Wait for loading indicator and response
          await mainWindow.waitForSelector('[data-testid="loading-indicator"]',
            { state: 'hidden', timeout: 30000 });

          // Get the latest response
          const response = await mainWindow.locator('[data-testid="message-container"]').last();
          const responseText = await response.textContent();
          expect(responseText).toBeTruthy();

          // Check for message history
          const messages = await mainWindow.locator('[data-testid="message-container"]').all();
          expect(messages.length).toBeGreaterThanOrEqual(2);

          // Take screenshot of chat history
          await mainWindow.screenshot({ path: `test-results/${provider.name.toLowerCase()}-chat-history.png` });

          // Test command history (up arrow) - re-query for the input since the element may have been re-rendered
          const chatInputForHistory = await mainWindow.waitForSelector('[data-testid="chat-input"]');
          await chatInputForHistory.press('Control+ArrowUp');
          const inputValue = await chatInputForHistory.inputValue();
          expect(inputValue).toBe('What is 2+2?');
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
          await mainWindow.locator('#extension-running-quotes').waitFor({ timeout: 30000 });
          await expect(
            mainWindow.locator('#extension-running-quotes button[role="switch"][data-state="checked"]')
          ).toBeVisible({ timeout: 10000 });

          // Navigate back to home
          await mainWindow.getByTestId('sidebar-home-button').click();

          // Send a message requesting a running quote
          const chatInput = mainWindow.getByTestId('chat-input');
          await chatInput.fill('Can you give me an inspirational running quote using the runningQuote tool?');
          await chatInput.press('Enter');

          // Wait for goose to finish responding
          await expect(mainWindow.getByTestId('loading-indicator')).toBeVisible({ timeout: 30000 });
          await expect(mainWindow.getByTestId('loading-indicator')).toBeHidden({ timeout: 30000 });

          // Verify the response contains a known quote
          const lastMessage = mainWindow.locator('.goose-message').last();
          const outputText = await lastMessage.textContent();

          const containsKnownQuote = runningQuotes.some(({ quote, author }) =>
            outputText.includes(`"${quote}" - ${author}`)
          );
          expect(containsKnownQuote).toBe(true);
      });
    });
  }
});
