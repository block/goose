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
  testPath: string;
};

// Create test fixture type
type TestFixtures = {
  provider: Provider;
};

// Define available providers
const providers: Provider[] = [
  { name: 'Databricks', testPath: 'div:has(h3:text("Databricks"))[class*="relative bg-bgApp rounded-lg"]' },
  { name: 'Google', testPath: 'div:has(h3:text("Google"))[class*="relative bg-bgApp rounded-lg"]' }
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
  
  // Click the menu button (3 dots) if we're in chat
  try {
    // Wait for header and menu button to be visible
    await mainWindow.waitForSelector('div[class*="bg-bgSubtle border-b border-borderSubtle"]', { timeout: 5000 });
    await mainWindow.waitForTimeout(1000); // Give UI time to stabilize
    
    const menuButton = await mainWindow.waitForSelector('button[aria-label="More options"]', { 
      timeout: 2000,
      state: 'visible'
    });
    await menuButton.click();
    
    // Wait for menu to be visible
    await mainWindow.waitForTimeout(1000);
    
    // Click "Reset provider and model"
    const resetProviderButton = await mainWindow.waitForSelector('button:has-text("Reset provider and model")', { timeout: 2000 });
    await resetProviderButton.click();

    // Wait for page to start refreshing
    await mainWindow.waitForTimeout(1000);

    // Wait for page to finish reloading
    await mainWindow.reload();
    await mainWindow.waitForLoadState('networkidle');
    
  } catch (e) {
    console.log('Already on provider selection screen or error:', e);
  }

  // Wait for provider selection screen
  const heading = await mainWindow.waitForSelector('h2:has-text("Choose a Provider")', { timeout: 2000 });
  const headingText = await heading.textContent();
  expect(headingText).toBe('Choose a Provider');

  // Find and verify the provider card container
  console.log(`Looking for ${provider.name} card...`);
  const providerContainer = await mainWindow.waitForSelector(provider.testPath);
  expect(await providerContainer.isVisible()).toBe(true);

  // Find the Launch button within the provider container
  console.log(`Looking for Launch button in ${provider.name} card...`);
  const launchButton = await providerContainer.waitForSelector('button:has-text("Launch")');
  expect(await launchButton.isVisible()).toBe(true);

  // Take screenshot before clicking
  await mainWindow.screenshot({ path: `test-results/before-${provider.name.toLowerCase()}-click.png` });

  // Click the Launch button
  await launchButton.click();

  // Wait for chat interface to appear
  const chatTextarea = await mainWindow.waitForSelector('textarea[placeholder*="What can goose help with?"]',
    { timeout: 5000 });
  expect(await chatTextarea.isVisible()).toBe(true);

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
        NODE_ENV: 'development'
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
        NODE_ENV: 'development'
      },
      recordVideo: {
        dir: 'test-results/videos/',
        size: { width: 620, height: 680 }
      }
    });

    // Get the main window once for all tests
    mainWindow = await electronApp.firstWindow();
    await mainWindow.waitForLoadState('domcontentloaded');
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

      await selectProvider(mainWindow, providers[0]);
  
      // Click the three dots menu button in the top right
      await mainWindow.waitForSelector('div[class*="bg-bgSubtle border-b border-borderSubtle"]');
      const menuButton = await mainWindow.waitForSelector('button[aria-label="More options"]', {
        timeout: 5000,
        state: 'visible'
      });
      await menuButton.click();
  
      // Find and click the dark mode toggle button
      const darkModeButton = await mainWindow.waitForSelector('button:has-text("Dark")');
      const lightModeButton = await mainWindow.waitForSelector('button:has-text("Light")');
      const systemModeButton = await mainWindow.waitForSelector('button:has-text("System")');

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
          const chatInput = await mainWindow.waitForSelector('textarea[placeholder*="What can goose help with?"]');
          expect(await chatInput.isVisible()).toBe(true);
    
          // Type a message
          await chatInput.fill('Hello, can you help me with a simple task?');
    
          // Take screenshot before sending
          await mainWindow.screenshot({ path: `test-results/${provider.name.toLowerCase()}-before-send.png` });
    
          // Get initial message count
          const initialMessages = await mainWindow.locator('.prose').count();
    
          // Send message
          await chatInput.press('Enter');
    
          // Wait for loading indicator to appear
          console.log('Waiting for loading indicator...');
          const loadingGoose = await mainWindow.waitForSelector('.text-textStandard >> text="goose is working on it…"',
            { timeout: 10000 });
          expect(await loadingGoose.isVisible()).toBe(true);
    
          // Take screenshot of loading state
          await mainWindow.screenshot({ path: `test-results/${provider.name.toLowerCase()}-loading-state.png` });
    
          // Wait for loading indicator to disappear
          console.log('Waiting for response...');
          await mainWindow.waitForSelector('.text-textStandard >> text="goose is working on it…"',
            { state: 'hidden', timeout: 30000 });
    
          // Wait for new message to appear
          await mainWindow.waitForFunction((count) => {
            const messages = document.querySelectorAll('.prose');
            return messages.length > count;
          }, initialMessages, { timeout: 30000 });
    
          // Get the latest response
          const response = await mainWindow.locator('.prose').last();
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
          const chatInput = await mainWindow.waitForSelector('textarea[placeholder*="What can goose help with?"]');
    
          // Test message sending with a specific question
          await chatInput.fill('What is 2+2?');
    
          // Get initial message count
          const initialMessages = await mainWindow.locator('.prose').count();
    
          // Send message
          await chatInput.press('Enter');
    
          // Wait for loading indicator and response
          await mainWindow.waitForSelector('.text-textStandard >> text="goose is working on it…"',
            { state: 'hidden', timeout: 30000 });
    
          // Wait for new message
          await mainWindow.waitForFunction((count) => {
            const messages = document.querySelectorAll('.prose');
            return messages.length > count;
          }, initialMessages, { timeout: 30000 });
    
          // Get the latest response
          const response = await mainWindow.locator('.prose').last();
          const responseText = await response.textContent();
          expect(responseText).toBeTruthy();
    
          // Check for message history
          const messages = await mainWindow.locator('.prose').all();
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
      
          // Clean up any existing running-quotes extensions from localStorage
          await mainWindow.evaluate(() => {
            const USER_SETTINGS_KEY = 'user_settings';
            const settings = JSON.parse(localStorage.getItem(USER_SETTINGS_KEY) || '{"extensions":[]}');
            
            // Remove any running-quotes extensions
            settings.extensions = settings.extensions.filter(ext => ext.id !== 'running-quotes');
            
            // Save back to localStorage
            localStorage.setItem(USER_SETTINGS_KEY, JSON.stringify(settings));
            
            // Log the cleanup
            console.log('Cleaned up existing running-quotes extensions');
          });
      
          // Reload the page to ensure settings are fresh
          await mainWindow.reload();
          await mainWindow.waitForLoadState('networkidle');
      
          // Debug: Print HTML structure before trying to find menu button
          // console.log('Debug: Current HTML structure before menu button selection:');
          // const htmlStructure = await mainWindow.evaluate(() => document.documentElement.outerHTML);
          // console.log(htmlStructure);

          // Take screenshot before attempting to find menu button
          await mainWindow.screenshot({ path: `test-results/${provider.name.toLowerCase()}-before-menu-button.png` });

          // Click the menu button (3 dots)
          console.log('Attempting to find menu button...');
          const menuButton = await mainWindow.waitForSelector('button[aria-label="More options"]', {
            timeout: 5000,
            state: 'visible'
          });
          console.log('Menu button found, clicking...');
          await menuButton.click();
      
          // Click Advanced settings
          const advancedSettingsButton = await mainWindow.waitForSelector('button:has-text("Advanced settings")');
          await advancedSettingsButton.click();
      
          // Wait for settings page and take screenshot
          await mainWindow.screenshot({ path: `test-results/${provider.name.toLowerCase()}-mcp-settings-page.png` });
      
          // Click Add Custom Extension button and wait for modal
          const addExtensionButton = await mainWindow.waitForSelector('button:has-text("Add Custom Extension")');
          await addExtensionButton.click();
      
          // Wait for modal and form to be fully rendered
          await mainWindow.waitForSelector('form', { state: 'visible', timeout: 10000 });
          console.log('Form found, waiting for modal animation...');
          await mainWindow.waitForTimeout(1000); // Wait for modal animation
      
          try {
            // Fill ID (find by label text)
            console.log('Filling ID field...');
            await mainWindow.locator('label:has-text("ID *") + input[type="text"]').fill('running-quotes');
      
            // Fill Name (find by label text)
            console.log('Filling Name field...');
            await mainWindow.locator('label:has-text("Name *") + input[type="text"]').fill('Running Quotes');
      
            // Fill Description (find by label text)
            console.log('Filling Description field...');
            await mainWindow.locator('label:has-text("Description *") + input[type="text"]').fill('Inspirational running quotes MCP server');
      
            // Fill Command (find by label text and placeholder)
            console.log('Filling Command field...');
            const mcpScriptPath = join(__dirname, 'basic-mcp.ts');
            await mainWindow.locator('label:has-text("Command *") + input[placeholder="e.g. goosed mcp example"]')
              .fill(`node ${mcpScriptPath}`);
      
            // Take screenshot of filled form
            await mainWindow.screenshot({ path: `test-results/${provider.name.toLowerCase()}-mcp-form-filled.png` });
      
            // Add a delay to inspect the form
            console.log('Waiting 5 seconds to inspect form...');
            await mainWindow.waitForTimeout(5000);
      
            // Click Add button (it's a submit button)
            console.log('Clicking Add button...');
            await mainWindow.locator('button[type="submit"]').click();
      
            // Wait for success toast and take screenshot
            await mainWindow.waitForSelector('.Toastify__toast-body div div:has-text("Successfully enabled extension")',
              { state: 'visible', timeout: 10000 });
            await mainWindow.screenshot({ path: `test-results/${provider.name.toLowerCase()}-mcp-extension-added.png` });
            console.log('Extension added successfully');
      
            // Click Exit button to return to chat
            const exitButton = await mainWindow.waitForSelector('button:has-text("Back")', { timeout: 5000 });
            await exitButton.click();
      
          } catch (error) {
            // Take error screenshot
            await mainWindow.screenshot({ path: `test-results/${provider.name.toLowerCase()}-mcp-form-error.png` });
            console.error('Error during form filling:', error);
            throw error;
          }
        });
      
        test('test running quotes functionality', async () => {
          console.log(`Testing running quotes functionality with ${provider.name}...`);
      
          // Find the chat input
          const chatInput = await mainWindow.waitForSelector('textarea[placeholder*="What can goose help with?"]');
          expect(await chatInput.isVisible()).toBe(true);
      
          // Type a message requesting a running quote
          await chatInput.fill('Can you give me an inspirational running quote using the runningQuote tool?');
      
          // Take screenshot before sending
          await mainWindow.screenshot({ path: `test-results/${provider.name.toLowerCase()}-before-quote-request.png` });
      
          // Get initial message count
          const initialMessages = await mainWindow.locator('.prose').count();
      
          // Send message
          await chatInput.press('Enter');
      
          // Wait for loading indicator
          const loadingIndicator = await mainWindow.waitForSelector('.text-textStandard >> text="goose is working on it…"',
            { timeout: 30000 });
          expect(await loadingIndicator.isVisible()).toBe(true);
      
          // Take screenshot of loading state
          await mainWindow.screenshot({ path: `test-results/${provider.name.toLowerCase()}-quote-loading.png` });
      
          // Wait for loading indicator to disappear
          await mainWindow.waitForSelector('.text-textStandard >> text="goose is working on it…"',
            { state: 'hidden', timeout: 30000 });
      
          // Wait for new message to appear
          await mainWindow.waitForFunction((count) => {
            const messages = document.querySelectorAll('.prose');
            return messages.length > count;
          }, initialMessages, { timeout: 30000 });
      
          // Get the latest response
          const response = await mainWindow.locator('.prose').last();
          expect(await response.isVisible()).toBe(true);
      
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
