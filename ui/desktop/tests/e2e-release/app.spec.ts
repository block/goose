import { test as base, expect } from '@playwright/test';
import { _electron as electron } from '@playwright/test';
import { join } from 'path';
import { exec, execSync } from 'child_process';
import { promisify } from 'util';

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
// Store screenshots directory
let screenshotsDir;

// Helper function to take a screenshot with error handling
async function takeScreenshot(name: string) {
  if (!mainWindow || !screenshotsDir) return;
  try {
    const path = join(screenshotsDir, `${name}.png`);
    console.log(`Taking screenshot: ${path}`);
    await mainWindow.screenshot({ path });
    console.log(`Screenshot saved: ${path}`);
  } catch (error) {
    console.error(`Failed to take screenshot ${name}:`, error);
  }
}

test.describe('Goose App Release Tests', () => {
  let electronApp;

  test.beforeAll(async () => {
    console.log('Starting Electron app...');

    // Create screenshots directory
    const fs = require('fs');
    screenshotsDir = join(__dirname, '../../test-results-release');
    if (!fs.existsSync(screenshotsDir)) {
      fs.mkdirSync(screenshotsDir, { recursive: true });
    }
    console.log('Screenshots directory:', screenshotsDir);

    // Get paths to the built app
    const appPath = join(__dirname, '../../out/Goose-darwin-arm64/Goose.app');
    const macOSBinary = join(appPath, 'Contents/MacOS/Goose');
    const resourcesPath = join(appPath, 'Contents/Resources');
    const asarPath = join(resourcesPath, 'app.asar');

    // Log paths and existence
    console.log('Checking paths:');
    console.log('App path:', appPath, 'exists:', fs.existsSync(appPath));
    console.log('MacOS binary:', macOSBinary, 'exists:', fs.existsSync(macOSBinary));
    console.log('Resources path:', resourcesPath, 'exists:', fs.existsSync(resourcesPath));
    console.log('Asar path:', asarPath, 'exists:', fs.existsSync(asarPath));

    // List contents of out directory
    console.log('\nContents of out directory:');
    const outDir = join(__dirname, '../../out');
    if (fs.existsSync(outDir)) {
      console.log(execSync('ls -la ' + outDir, { encoding: 'utf8' }));
    } else {
      console.log('out directory does not exist');
    }

    // List contents of app bundle
    if (fs.existsSync(appPath)) {
      console.log('\nContents of app bundle:');
      console.log(execSync('ls -la ' + appPath, { encoding: 'utf8' }));
    }

    // Launch electron with our app
    try {
      console.log('\nLaunching electron app...');
      const macOSBinary = join(__dirname, '../../out/Goose-darwin-arm64/Goose.app/Contents/MacOS/Goose');
      
      electronApp = await electron.launch({
        executablePath: macOSBinary,
        env: {
          GOOSE_ALLOWLIST_BYPASS: 'true',
          GOOSE_TEST_MODE: 'true',
          ELECTRON_ENABLE_LOGGING: '1',
          ELECTRON_ENABLE_STACK_DUMPING: '1',
          DEBUG: '*',
          ELECTRON_IS_PACKAGED: 'true', // Tell the app it's packaged
          GOOSE_E2E_TEST: 'true' // Custom flag for E2E testing of packaged app
        },
        timeout: 45000 // Increase timeout for packaged app launch
      });
      console.log('Electron app launched successfully');
    } catch (error) {
      console.error('Failed to launch electron app:', error);
      throw error;
    }

    // Get the main window
    try {
      console.log('Waiting for main window...');
      mainWindow = await electronApp.firstWindow({ timeout: 45000 });
      console.log('Main window obtained');

      console.log('Waiting for domcontentloaded...');
      await mainWindow.waitForLoadState('domcontentloaded');
      console.log('domcontentloaded complete');

      console.log('Waiting for networkidle...');
      await mainWindow.waitForLoadState('networkidle');
      console.log('networkidle complete');

      // Take initial screenshot
      await takeScreenshot('initial-state');

      // Log current HTML to help debug selectors
      const html = await mainWindow.evaluate(() => document.documentElement.outerHTML);
      console.log('Current page HTML:', html);

      // Ensure window is visible and focused
      await mainWindow.bringToFront();
      
      // Set window bounds to ensure it's visible
      await mainWindow.setViewportSize({ width: 1024, height: 768 });

      // Wait for React app to be ready by looking for any of the known elements
      console.log('Waiting for app elements...');
      
      // Wait for root element to exist
      console.log('Waiting for root element...');
      await mainWindow.waitForSelector('#root', { timeout: 30000 });
      console.log('Root element found');

      // Wait for root to be populated
      console.log('Waiting for root element to be populated...');
      await mainWindow.waitForFunction(() => {
        const root = document.getElementById('root');
        return root && root.children.length > 0;
      }, { timeout: 30000 });
      console.log('Root element populated');

      // Take screenshot after root is populated
      await takeScreenshot('root-populated');

      // Wait for any of our known elements
      console.log('Waiting for app UI elements...');
      await Promise.race([
        mainWindow.waitForSelector('[data-testid="provider-selection-heading"]', { timeout: 30000 }),
        mainWindow.waitForSelector('[data-testid="chat-input"]', { timeout: 30000 }),
        mainWindow.waitForSelector('[data-testid="more-options-button"]', { timeout: 30000 })
      ]);
      console.log('Found app elements');

      // Take another screenshot
      await takeScreenshot('after-wait');
    } catch (error) {
      console.error('Error during window initialization:', error);
      // Take screenshot of the current state if we have a window
      if (mainWindow) {
        await takeScreenshot('init-error');
      }
      throw error;
    }
  });

  test.afterAll(async () => {
    console.log('Final cleanup...');

    // Take final screenshot if we have a window
    if (mainWindow) {
      await takeScreenshot('final-state');
    }

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
  });

  test.describe('General UI', () => {
    test('dark mode toggle', async () => {
      console.log('Testing dark mode toggle...');

      // Take initial screenshot to see what state we're in
      await takeScreenshot('initial-test-state');

      try {
        // Select the default provider
        await selectProvider(mainWindow, providers[0]);
      } catch (error) {
        console.error('Error selecting provider:', error);
        // Take error screenshot
        await takeScreenshot('provider-select-error');
        throw error;
      }
  
      // Click the three dots menu button in the top right
      const menuButton = await mainWindow.waitForSelector('[data-testid="more-options-button"]', {
        timeout: 5000,
        state: 'visible'
      });
      await menuButton.click();
      await takeScreenshot('menu-open');
  
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
        await takeScreenshot('dark-mode-toggle');
      } else {
        // Click to toggle to dark mode
        await darkModeButton.click();
        await mainWindow.waitForTimeout(1000);
        const newDarkMode = await mainWindow.evaluate(() => document.documentElement.classList.contains('dark'));
        expect(newDarkMode).toBe(!isDarkMode);
        await takeScreenshot('dark-mode-toggle');
      }

      // check that system mode is clickable
      await systemModeButton.click();
      await takeScreenshot('system-mode');
  
      // Toggle back to light mode
      await lightModeButton.click();
      
      // Pause to show return to original state
      await mainWindow.waitForTimeout(2000);
      await takeScreenshot('final-mode');
  
      // Close menu with ESC key
      await mainWindow.keyboard.press('Escape');
    });
  });
});

// Helper function to select a provider
async function selectProvider(mainWindow: any, provider: Provider) {
  console.log(`Selecting provider: ${provider.name}`);
  
  // Take screenshot of initial state
  await takeScreenshot('provider-select-start');

  // If we're already in the chat interface, we need to reset providers
  console.log('Checking for chat interface...');
  const chatTextarea = await mainWindow.waitForSelector('[data-testid="chat-input"]', { 
    timeout: 5000
  }).catch(() => null);

  if (chatTextarea) {
    console.log('Found chat interface, resetting provider...');
    await takeScreenshot('provider-select-chat-found');

    // Click menu button to reset providers
    console.log('Opening menu to reset providers...');
    const menuButton = await mainWindow.waitForSelector('[data-testid="more-options-button"]', {
      timeout: 5000,
      state: 'visible'
    });
    await menuButton.click();

    // Wait for menu to appear and be interactive
    await mainWindow.waitForTimeout(1000);
    await takeScreenshot('provider-select-menu-open');

    // Click Reset Provider and Model
    console.log('Clicking Reset provider and model...');
    const resetButton = await mainWindow.waitForSelector('button:has-text("Reset provider and model")', {
      timeout: 5000,
      state: 'visible'
    });
    await resetButton.click();
    await takeScreenshot('provider-select-after-reset');
  }

  // Wait for React app to be ready and animations to complete
  console.log('Waiting for provider selection screen...');
  await mainWindow.waitForTimeout(2000);

  // We should now be at provider selection
  const providerHeading = await mainWindow.waitForSelector('[data-testid="provider-selection-heading"]', {
    timeout: 10000
  });
  console.log('Found provider selection heading:', await providerHeading.textContent());

  // Take screenshot before looking for provider card
  await takeScreenshot('provider-select-before-card');

  // Find and verify the provider card container
  console.log(`Looking for ${provider.name} card...`);
  let providerContainer;
  try {
    providerContainer = await mainWindow.waitForSelector(`[data-testid="provider-card-${provider.name.toLowerCase()}"]`, {
      timeout: 10000
    });
    expect(await providerContainer.isVisible()).toBe(true);
    console.log('Found provider card');
  } catch (error) {
    console.error(`Provider card not found for ${provider.name}. This could indicate a missing or incorrectly configured provider.`);
    await takeScreenshot('provider-select-card-error');
    throw error;
  }

  // Find the Launch button within the provider container
  console.log(`Looking for Launch button in ${provider.name} card...`);
  const launchButton = await providerContainer.waitForSelector('[data-testid="provider-launch-button"]', {
    timeout: 10000
  });
  expect(await launchButton.isVisible()).toBe(true);
  console.log('Found launch button');

  // Take screenshot before clicking
  await takeScreenshot('provider-select-before-launch');

  // Click the Launch button
  await launchButton.click();
  console.log('Clicked launch button');

  // Wait for chat interface to appear
  console.log('Waiting for chat interface...');
  const chatTextareaAfterClick = await mainWindow.waitForSelector('[data-testid="chat-input"]', {
    timeout: 10000
  });
  expect(await chatTextareaAfterClick.isVisible()).toBe(true);
  console.log('Found chat interface');

  // Take screenshot of chat interface
  await takeScreenshot('provider-select-complete');
}