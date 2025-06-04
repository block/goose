import { test as base, expect } from '@playwright/test';
import { _electron as electron } from '@playwright/test';
import { join } from 'path';
import { spawn, exec } from 'child_process';
import { promisify } from 'util';
import { showTestName, clearTestName } from './test-overlay';

const { runningQuotes } = require('./basic-mcp');
const execAsync = promisify(exec);

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
  { name: 'Databricks' },
  { name: 'Google' }
];

// Create test with fixtures
const test = base.extend<TestFixtures>({
  provider: [providers[0], { option: true }], // Default to first provider (Databricks)
});

// Store mainWindow reference
let mainWindow;

// Add hooks for test name overlay
// eslint-disable-next-line no-empty-pattern
test.beforeEach(async ({ }, testInfo) => {
  if (mainWindow) {
    // Get a clean test name without the full hierarchy
    const testName = testInfo.titlePath[testInfo.titlePath.length - 1];
    
    // Get provider name if we're in a provider suite
    const providerSuite = testInfo.titlePath.find(t => t.startsWith('Provider:'));
    const providerName = providerSuite ? providerSuite.split(': ')[1] : undefined;
    
    console.log(`Setting overlay for test: "${testName}"${providerName ? ` (Provider: ${providerName})` : ''}`);
    await showTestName(mainWindow, testName, providerName);
  }
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
    // Click menu button to reset providers
    console.log('Opening menu to reset providers...');
    const menuButton = await mainWindow.waitForSelector('[data-testid="more-options-button"]', {
      timeout: 5000,
      state: 'visible'
    });
    await menuButton.click();

    // Wait for menu to appear and be interactive
    await mainWindow.waitForTimeout(1000);

    // Click Reset Provider and Model
    console.log('Clicking Reset provider and model...');
    const resetButton = await mainWindow.waitForSelector('button:has-text("Reset provider and model")', {
      timeout: 5000,
      state: 'visible'
    });
    await resetButton.click();
  }

  // Wait for React app to be ready and animations to complete
  await mainWindow.waitForFunction(() => {
    const root = document.getElementById('root');
    return root && root.children.length > 0;
  });
  await mainWindow.waitForTimeout(2000);

  // Take a screenshot before proceeding
  await mainWindow.screenshot({ path: `test-results/before-provider-${provider.name.toLowerCase()}-check.png` });

  // We should now be at provider selection
  await mainWindow.waitForSelector('[data-testid="provider-selection-heading"]');

  // Find and verify the provider card container
  console.log(`Looking for ${provider.name} card...`);
  let providerContainer;
  try {
    providerContainer = await mainWindow.waitForSelector(`[data-testid="provider-card-${provider.name.toLowerCase()}"]`);
    expect(await providerContainer.isVisible()).toBe(true);
  } catch (error) {
    console.error(`Provider card not found for ${provider.name}. This could indicate a missing or incorrectly configured provider.`);
    throw error;
  }

  // Find the Launch button within the provider container
  console.log(`Looking for Launch button in ${provider.name} card...`);
  const launchButton = await providerContainer.waitForSelector('[data-testid="provider-launch-button"]');
  expect(await launchButton.isVisible()).toBe(true);

  // Take screenshot before clicking
  await mainWindow.screenshot({ path: `test-results/before-${provider.name.toLowerCase()}-click.png` });

  // Click the Launch button
  await launchButton.click();

  // Wait for chat interface to appear
  const chatTextareaAfterClick = await mainWindow.waitForSelector('[data-testid="chat-input"]',
    { timeout: 2000 });
  expect(await chatTextareaAfterClick.isVisible()).toBe(true);

  // Take screenshot of chat interface
  await mainWindow.screenshot({ path: `test-results/chat-interface-${provider.name.toLowerCase()}.png` });
}

test.describe('Goose App', () => {
  let electronApp;
  let appProcess;

  test.beforeAll(async () => {
    console.log('Starting Electron app...');

    // Start the electron-forge process
    appProcess = spawn('npm', ['run', 'start-gui'], {
      cwd: join(__dirname, '../..'),
      stdio: 'pipe',
      shell: true,
      env: {
        ...process.env,
        ELECTRON_IS_DEV: '1',
        NODE_ENV: 'development',
        GOOSE_ALLOWLIST_BYPASS: 'true',
      }
    });

    // Log process output
    appProcess.stdout.on('data', (data) => {
      console.log('App stdout:', data.toString());
    });

    appProcess.stderr.on('data', (data) => {
      console.log('App stderr:', data.toString());
    });

    // Wait a bit for the app to start
    console.log('Waiting for app to start...');
    await new Promise(resolve => setTimeout(resolve, 5000));

    // Launch Electron for testing
    electronApp = await electron.launch({
      args: ['.vite/build/main.js'],
      cwd: join(__dirname, '../..'),
      env: {
        ...process.env,
        ELECTRON_IS_DEV: '1',
        NODE_ENV: 'development',
      },
      recordVideo: {
        dir: 'test-results/videos/',
        size: { width: 620, height: 680 }
      }
    });

    // Get the main window once for all tests
    mainWindow = await electronApp.firstWindow();
    await mainWindow.waitForLoadState('domcontentloaded');
    
    // Try to wait for networkidle, but don't fail if it times out due to MCP activity
    try {
      await mainWindow.waitForLoadState('networkidle', { timeout: 10000 });
    } catch (error) {
      console.log('NetworkIdle timeout (likely due to MCP activity), continuing with test...');
    }

    // Wait for React app to be ready by checking for the root element to have content
    await mainWindow.waitForFunction(() => {
      const root = document.getElementById('root');
      return root && root.children.length > 0;
    });

    // Wait for any animations to complete
    await mainWindow.waitForTimeout(2000);

    // Take a screenshot to debug what's on the screen
    await mainWindow.screenshot({ path: 'test-results/initial-load.png' });

    // Debug: print out the page content
    const content = await mainWindow.content();
    console.log('Page content:', content);
  });

  test.afterAll(async () => {
    console.log('Final cleanup...');

    // Close the test instance
    if (electronApp) {
      await electronApp.close().catch(console.error);
    }

    // Kill any remaining electron processes
    try {
      if (process.platform === 'win32') {
        await execAsync('taskkill /F /IM electron.exe');
      } else {
        await execAsync('pkill -f electron || true');
      }
    } catch (error) {
      if (!error.message?.includes('no process found')) {
        console.error('Error killing electron processes:', error);
      }
    }

    // Kill any remaining npm processes from start-gui
    try {
      if (process.platform === 'win32') {
        await execAsync('taskkill /F /IM node.exe');
      } else {
        await execAsync('pkill -f "start-gui" || true');
      }
    } catch (error) {
      if (!error.message?.includes('no process found')) {
        console.error('Error killing npm processes:', error);
      }
    }

    // Kill the specific npm process if it's still running
    try {
      if (appProcess && appProcess.pid) {
        process.kill(appProcess.pid);
      }
    } catch (error) {
      if (error.code !== 'ESRCH') {
        console.error('Error killing npm process:', error);
      }
    }
  });

  test.describe('General UI', () => {
    test('dark mode toggle', async () => {
      console.log('Testing dark mode toggle...');

      const chatTextarea = await mainWindow.waitForSelector('[data-testid="chat-input"]', {
        timeout: 2000
      }).catch(() => null);
      if (!chatTextarea) {
        await selectProvider(mainWindow, providers[0]);
      }

      const menuButton = await mainWindow.waitForSelector('[data-testid="more-options-button"]', {
        timeout: 5000,
        state: 'visible'
      });
      await menuButton.click();
  
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
  
      // Close menu with ESC key
      await mainWindow.keyboard.press('Escape');
    });
  });

  for (const provider of providers) {
    test.describe(`Provider: ${provider.name}`, () => {
      test.beforeAll(async () => {
        // Select the provider once before all tests for this provider
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
    
          // Get initial message count
          const initialMessages = await mainWindow.locator('[data-testid="message-container"]').count();
    
          // Send message
          await chatInput.press('Enter');
    
          // Wait for loading indicator to appear
          console.log('Waiting for loading indicator...');
          const loadingGoose = await mainWindow.waitForSelector('[data-testid="loading-indicator"]',
            { timeout: 2000 });
          expect(await loadingGoose.isVisible()).toBe(true);
    
          // Take screenshot of loading state
          await mainWindow.screenshot({ path: `test-results/${provider.name.toLowerCase()}-loading-state.png` });
    
          // Wait for loading indicator to disappear
          console.log('Waiting for response...');
          await mainWindow.waitForSelector('[data-testid="loading-indicator"]',
            { state: 'hidden', timeout: 30000 });
    
          // Wait for new message to appear
          await mainWindow.waitForFunction((count) => {
            const messages = document.querySelectorAll('[data-testid="message-container"]');
            return messages.length > count;
          }, initialMessages, { timeout: 30000 });
    
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
    
          // Get initial message count
          const initialMessages = await mainWindow.locator('[data-testid="message-container"]').count();
    
          // Send message
          await chatInput.press('Enter');
    
          // Wait for loading indicator and response
          await mainWindow.waitForSelector('[data-testid="loading-indicator"]',
            { state: 'hidden', timeout: 30000 });
    
          // Wait for new message
          await mainWindow.waitForFunction((count) => {
            const messages = document.querySelectorAll('[data-testid="message-container"]');
            return messages.length > count;
          }, initialMessages, { timeout: 30000 });
    
          // Get the latest response
          const response = await mainWindow.locator('[data-testid="message-container"]').last();
          const responseText = await response.textContent();
          expect(responseText).toBeTruthy();
    
          // Check for message history
          const messages = await mainWindow.locator('[data-testid="message-container"]').all();
          expect(messages.length).toBeGreaterThanOrEqual(2);
    
          // Take screenshot of chat history
          await mainWindow.screenshot({ path: `test-results/${provider.name.toLowerCase()}-chat-history.png` });
    
          // Test command history (up arrow)
          await chatInput.press('Control+ArrowUp');
          const inputValue = await chatInput.inputValue();
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

            // First navigate to Advanced Settings to check for existing Running Quotes extension
            console.log('Checking for existing Running Quotes extension...');
            
            // Click the menu button (3 dots)
            const menuButton = await mainWindow.waitForSelector('[data-testid="more-options-button"]', {
              timeout: 2000,
              state: 'visible'
            });
            await menuButton.click();
            
            // Wait for menu to appear
            await mainWindow.waitForTimeout(1000);
            
            // Click Advanced settings
            const advancedSettingsButton = await mainWindow.waitForSelector('button:has-text("Advanced settings")', {
              timeout: 2000,
              state: 'visible'
            });
            await advancedSettingsButton.click();
            
            // Wait for settings page to load
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
            
            // Click Back to return to main menu
            let backButton = await mainWindow.waitForSelector('button:has-text("Back")', {
              timeout: 2000,
              state: 'visible'
            });
            await backButton.click();
            
            // Wait for menu transition
            await mainWindow.waitForTimeout(1000);
            
            // Now proceed with adding the extension
            console.log('Proceeding with adding Running Quotes extension...');

            // Find and click the menu button (3 dots) again
            console.log('Finding menu button again...');
            const menuButtonAgain = await mainWindow.waitForSelector('[data-testid="more-options-button"]', {
              timeout: 2000,
              state: 'visible'
            });
            await menuButtonAgain.click();
            
            // Wait for menu to appear and take screenshot
            await mainWindow.waitForTimeout(1000);
            await mainWindow.screenshot({ path: `test-results/${provider.name.toLowerCase()}-after-menu.png` });

            // Click Advanced settings again
            console.log('Looking for Advanced settings button again...');
            const advancedSettingsButtonAgain = await mainWindow.waitForSelector('button:has-text("Advanced settings")', {
              timeout: 2000,
              state: 'visible'
            });
            await advancedSettingsButtonAgain.click();
            console.log('Clicked Advanced settings');
            
            // Wait for navigation and take screenshot
            await mainWindow.waitForTimeout(1000);
            await mainWindow.screenshot({ path: `test-results/${provider.name.toLowerCase()}-settings-page.png` });

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
            const modalAddButton = await mainWindow.waitForSelector('button.text-textProminent', {
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

            // Click Back button
            backButton = await mainWindow.waitForSelector('button:has-text("Back")', { 
              timeout: 2000,
              state: 'visible'
            });
            await backButton.click();
            console.log('Clicked Back button');

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
          await chatInput.fill('Can you give me an inspirational running quote using the runningQuote tool?');
      
          // Take screenshot before sending
          await mainWindow.screenshot({ path: `test-results/${provider.name.toLowerCase()}-before-quote-request.png` });
      
          // Get initial message count
          const initialMessages = await mainWindow.locator('[data-testid="message-container"]').count();
      
          // Send message
          await chatInput.press('Enter');
      
          // Wait for loading indicator
          const loadingIndicator = await mainWindow.waitForSelector('[data-testid="loading-indicator"]',
            { timeout: 30000 });
          expect(await loadingIndicator.isVisible()).toBe(true);
      
          // Take screenshot of loading state
          await mainWindow.screenshot({ path: `test-results/${provider.name.toLowerCase()}-quote-loading.png` });
      
          // Wait for loading indicator to disappear
          await mainWindow.waitForSelector('[data-testid="loading-indicator"]',
            { state: 'hidden', timeout: 30000 });
      
          // Wait for new message to appear
          await mainWindow.waitForFunction((count) => {
            const messages = document.querySelectorAll('[data-testid="message-container"]');
            return messages.length > count;
          }, initialMessages, { timeout: 30000 });
      
          // Get the latest response
          const response = await mainWindow.waitForSelector('.goose-message-tool', { timeout: 5000 });
          expect(await response.isVisible()).toBe(true);
      
          // Click the Output dropdown to reveal the actual quote
                    await mainWindow.screenshot({ path: `test-results/${provider.name.toLowerCase()}-quote-response-debug.png` });
          const element = await mainWindow.$('.goose-message-tool');
          const html = await element.innerHTML();
          console.log('HTML content:', html);
          // Click the Runningquote dropdown to reveal the actual quote
          const runningQuoteButton = await mainWindow.waitForSelector('div.goose-message-tool svg.rotate-90', { timeout: 5000 });
          await runningQuoteButton.click();

          // Click the Output dropdown to reveal the actual quote
          const outputButton = await mainWindow.waitForSelector('button:has-text("Output")', { timeout: 5000 });
          await outputButton.click();
      
          // Wait a bit and dump HTML to see structure
          await mainWindow.waitForTimeout(1000);
      
          // Take screenshot before trying to find content
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
