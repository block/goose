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

  // If we're already in the chat interface, we need to reset providers
  const chatTextarea = await mainWindow.waitForSelector('[data-testid="chat-input"]', {
    timeout: 2000
  }).catch(() => null);

  if (chatTextarea) {
    // Navigate to Settings via sidebar to reset providers
    console.log('Opening settings to reset providers...');
    const settingsButton = await mainWindow.waitForSelector('[data-testid="sidebar-settings-button"]', {
      timeout: 5000,
      state: 'visible'
    });
    await settingsButton.click();

    // Wait for settings page to load and navigate to Models tab
    await mainWindow.waitForSelector('[data-testid="settings-models-tab"]', {
      timeout: 5000,
      state: 'visible'
    });

    const modelsTab = await mainWindow.waitForSelector('[data-testid="settings-models-tab"]');
    await modelsTab.click();

    // Wait for models section to load
    await mainWindow.waitForTimeout(1000);

    // Click Reset Provider and Model button
    console.log('Clicking Reset provider and model...');
    const resetButton = await mainWindow.waitForSelector('button:has-text("Reset provider and model")', {
      timeout: 5000,
      state: 'visible'
    });
    await resetButton.click();

    // Wait for the reset to complete
    await mainWindow.waitForTimeout(1000);
  }

  // Wait for React app to be ready and animations to complete
  await mainWindow.waitForFunction(() => {
    const root = document.getElementById('root');
    return root && root.children.length > 0;
  });
  await mainWindow.waitForTimeout(10000);

  // Take a screenshot before proceeding
  await mainWindow.screenshot({ path: `test-results/before-provider-${provider.name.toLowerCase()}-check.png` });

  // Check if we're already at the chat interface (provider already configured)
  const chatInputAfterReset = await mainWindow.waitForSelector('[data-testid="chat-input"]', {
    timeout: 2000,
    state: 'visible'
  }).catch(() => null);

  if (chatInputAfterReset) {
    console.log('Provider already configured, chat interface is available');
    return; // Provider is already selected, no need to do anything
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

      test.describe('MCP Integration', () => {
        test('running quotes MCP server integration', async () => {
          console.log(`Testing Running Quotes MCP server integration with ${provider.name}...`);

          // Create test-results directory if it doesn't exist
          const fs = require('fs');
          if (!fs.existsSync('test-results')) {
            fs.mkdirSync('test-results', { recursive: true });
          }

          try {
            // Reload the page to ensure settings are fresh
            await mainWindow.reload();
            // Try to wait for networkidle, but don't fail if it times out due to MCP activity
            try {
              await mainWindow.waitForLoadState('networkidle', { timeout: 10000 });
            } catch (error) {
              console.log('NetworkIdle timeout (likely due to MCP activity), continuing with test...');
            }
            await mainWindow.waitForLoadState('domcontentloaded');

            // Wait for React app to be ready
            await mainWindow.waitForFunction(() => {
              const root = document.getElementById('root');
              return root && root.children.length > 0;
            });

            // Take screenshot of initial state
            await mainWindow.screenshot({ path: `test-results/${provider.name.toLowerCase()}-initial-state.png` });

            // Navigate to Extensions via sidebar
            console.log('Navigating to Extensions...');
            const extensionsButton = await mainWindow.waitForSelector('[data-testid="sidebar-extensions-button"]', {
              timeout: 5000,
              state: 'visible'
            });
            await extensionsButton.click();

            // Wait for extensions page to load
            await mainWindow.waitForTimeout(1000);

            // Look for Running Quotes extension card
            console.log('Looking for existing Running Quotes extension...');
            const existingExtension = await mainWindow.$('div.flex:has-text("Running Quotes")');

            if (existingExtension) {
              console.log('Found existing Running Quotes extension, removing it...');

              // Find and click the settings gear icon next to Running Quotes
              const settingsButton = await existingExtension.$('button[aria-label="Extension settings"]');
              if (settingsButton) {
                await settingsButton.click();

                // Wait for modal to appear
                await mainWindow.waitForTimeout(500);

                // Click the Remove Extension button
                const removeButton = await mainWindow.waitForSelector('button:has-text("Remove Extension")', {
                  timeout: 2000,
                  state: 'visible'
                });
                await removeButton.click();

                // Wait for confirmation modal
                await mainWindow.waitForTimeout(500);

                // Click the Remove button in confirmation dialog
                const confirmButton = await mainWindow.waitForSelector('button:has-text("Remove")', {
                  timeout: 2000,
                  state: 'visible'
                });
                await confirmButton.click();

                // Wait for extension to be removed
                await mainWindow.waitForTimeout(1000);
              }
            }

            // Now proceed with adding the extension
            console.log('Proceeding with adding Running Quotes extension...');

            // Click "Add custom extension" button
            console.log('Looking for Add custom extension button...');
            const addExtensionButton = await mainWindow.waitForSelector('button:has-text("Add custom extension")', {
              timeout: 2000,
              state: 'visible'
            });

            // Verify add extension button is visible
            const isAddExtensionVisible = await addExtensionButton.isVisible();
            console.log('Add custom extension button visible:', isAddExtensionVisible);

            await addExtensionButton.click();
            console.log('Clicked Add custom extension');

            // Wait for modal and take screenshot
            await mainWindow.waitForTimeout(1000);
            await mainWindow.screenshot({ path: `test-results/${provider.name.toLowerCase()}-modal.png` });

            // Fill the form
            console.log('Filling form fields...');

            // Fill Extension Name
            const nameInput = await mainWindow.waitForSelector('input[placeholder="Enter extension name..."]', {
              timeout: 2000,
              state: 'visible'
            });
            await nameInput.fill('Running Quotes');

            // Fill Description
            const descriptionInput = await mainWindow.waitForSelector('input[placeholder="Optional description..."]', {
              timeout: 2000,
              state: 'visible'
            });
            await descriptionInput.fill('Inspirational running quotes MCP server');

            // Fill Command
            const mcpScriptPath = join(__dirname, 'basic-mcp.ts');
            const commandInput = await mainWindow.waitForSelector('input[placeholder="e.g. npx -y @modelcontextprotocol/my-extension <filepath>"]', {
              timeout: 2000,
              state: 'visible'
            });
            await commandInput.fill(`node ${mcpScriptPath}`);

            // Take screenshot of filled form
            await mainWindow.screenshot({ path: `test-results/${provider.name.toLowerCase()}-filled-form.png` });

            // Wait for any animations to complete
            await mainWindow.waitForTimeout(1000);

            // Click Add Extension button in modal footer
            console.log('Looking for Add Extension button in modal...');
            const modalAddButton = await mainWindow.waitForSelector('[data-testid="extension-submit-btn"]', {
              timeout: 2000,
              state: 'visible'
            });

            // Verify button is visible
            const isModalAddButtonVisible = await modalAddButton.isVisible();
            console.log('Add Extension button visible:', isModalAddButtonVisible);

            // Click the button
            await modalAddButton.click();

            console.log('Clicked Add Extension button');

            // Wait for the Running Quotes extension to appear in the list
            console.log('Waiting for Running Quotes extension to appear...');
            try {
              const extensionCard = await mainWindow.waitForSelector(
                'div.flex:has-text("Running Quotes")',
                {
                  timeout: 30000,
                  state: 'visible'
                }
              );

              // Verify the extension is enabled
              await mainWindow.waitForTimeout(1000);
              const toggleButton = await extensionCard.$('button[role="switch"][data-state="checked"]');
              const isEnabled = !!toggleButton;
              console.log('Extension enabled:', isEnabled);

              if (!isEnabled) {
                throw new Error('Running Quotes extension was added but not enabled');
              }

              await mainWindow.screenshot({ path: `test-results/${provider.name.toLowerCase()}-extension-added.png` });
              console.log('Extension added successfully');
            } catch (error) {
              console.error('Error verifying extension:', error);

              // Get any error messages that might be visible
              const errorElements = await mainWindow.$$eval('.text-red-500, .text-error',
                elements => elements.map(el => el.textContent)
              );
              if (errorElements.length > 0) {
                console.log('Found error messages:', errorElements);
              }

              throw error;
            }

            // Navigate back to home
            const homeButton = await mainWindow.waitForSelector('[data-testid="sidebar-home-button"]');
            await homeButton.click();
            console.log('Navigated back to home');

          } catch (error) {
            // Take error screenshot and log details
            await mainWindow.screenshot({ path: `test-results/${provider.name.toLowerCase()}-error.png` });

            // Get page content
            const pageContent = await mainWindow.evaluate(() => document.body.innerHTML);
            console.log('Page content at error:', pageContent);

            console.error('Test failed:', error);
            throw error;
          }
        });

        test('test running quotes functionality', async () => {
          console.log(`Testing running quotes functionality with ${provider.name}...`);

          // Find the chat input
          const chatInput = await mainWindow.waitForSelector('[data-testid="chat-input"]');
          expect(await chatInput.isVisible()).toBe(true);

          // Type a message requesting a running quote
          await chatInput.fill('Can you give me an inspirational running quote using the runningQuotes tool?');

          // Take screenshot before sending
          await mainWindow.screenshot({ path: `test-results/${provider.name.toLowerCase()}-before-quote-request.png` });

          // Send message
          await chatInput.press('Enter');

          // Get the latest response
          const response = await mainWindow.waitForSelector('.goose-message-tool', { timeout: 5000 });
          expect(await response.isVisible()).toBe(true);

          // Click the Output dropdown to reveal the actual quote
          await mainWindow.screenshot({ path: `test-results/${provider.name.toLowerCase()}-quote-response-debug.png` });

          // Now try to get the output content
          const outputContent = await mainWindow.waitForSelector('.whitespace-pre-wrap', { timeout: 5000 });
          const outputText = await outputContent.textContent();
          console.log('Output text:', outputText);

          // Take screenshot of expanded response
          await mainWindow.screenshot({ path: `test-results/${provider.name.toLowerCase()}-quote-response.png` });

          // Check if the output contains one of our known quotes
          const containsKnownQuote = runningQuotes.some(({ quote, author }) =>
            outputText.includes(`"${quote}" - ${author}`)
          );
          expect(containsKnownQuote).toBe(true);
        });
      });
    });
  }
});
