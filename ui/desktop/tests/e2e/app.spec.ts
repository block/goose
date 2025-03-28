import { test, expect } from '@playwright/test';
import { _electron as electron } from '@playwright/test';
import { join } from 'path';
import { spawn, exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

test.describe('Goose App', () => {
  let electronApp;
  let appProcess;
  let mainWindow;
  let isProviderSelected = false;

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
      }
    });

    // Get the main window once for all tests
    mainWindow = await electronApp.firstWindow();
    await mainWindow.waitForLoadState('domcontentloaded');

    // Check if we're already on the chat screen
    try {
      const chatInput = await mainWindow.waitForSelector('textarea[placeholder*="What can goose help with?"]', 
        { timeout: 5000 });
      isProviderSelected = await chatInput.isVisible();
      console.log('Provider already selected, chat interface visible');
    } catch (e) {
      console.log('On provider selection screen');
      isProviderSelected = false;
    }
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

  test('verify initial screen and select provider if needed', async () => {
    console.log('Checking initial screen state...');

    if (!isProviderSelected) {
      // Take screenshot of provider selection screen
      await mainWindow.screenshot({ path: 'test-results/provider-selection.png' });
      
      // Verify provider selection screen
      const heading = await mainWindow.waitForSelector('h2:has-text("Choose a Provider")', { timeout: 10000 });
      const headingText = await heading.textContent();
      expect(headingText).toBe('Choose a Provider');
      
      // Find and verify the Databricks card container
      console.log('Looking for Databricks card...');
      const databricksContainer = await mainWindow.waitForSelector(
        'div:has(h3:text("Databricks"))[class*="relative bg-bgApp rounded-lg"]'
      );
      expect(await databricksContainer.isVisible()).toBe(true);
      
      // Find the Launch button within the Databricks container
      console.log('Looking for Launch button in Databricks card...');
      const launchButton = await databricksContainer.waitForSelector('button:has-text("Launch")');
      expect(await launchButton.isVisible()).toBe(true);
      
      // Take screenshot before clicking
      await mainWindow.screenshot({ path: 'test-results/before-databricks-click.png' });
      
      // Click the Launch button
      await launchButton.click();
      
      // Wait for chat interface to appear
      const chatTextarea = await mainWindow.waitForSelector('textarea[placeholder*="What can goose help with?"]', 
        { timeout: 15000 });
      expect(await chatTextarea.isVisible()).toBe(true);
    } else {
      console.log('Provider already selected, skipping provider selection test');
    }
    
    // Take screenshot of current state
    await mainWindow.screenshot({ path: 'test-results/chat-interface.png' });
  });

  test('chat interaction', async () => {
    console.log('Testing chat interaction...');
    
    // Find the chat input
    const chatInput = await mainWindow.waitForSelector('textarea[placeholder*="What can goose help with?"]');
    expect(await chatInput.isVisible()).toBe(true);
    
    // Type a message
    await chatInput.fill('Hello, can you help me with a simple task?');
    
    // Take screenshot before sending
    await mainWindow.screenshot({ path: 'test-results/before-send.png' });
    
    // Get initial message count
    const initialMessages = await mainWindow.locator('.prose').count();
    
    // Send message
    await chatInput.press('Enter');
    
    // Wait for loading indicator to appear (using the specific class and text)
    console.log('Waiting for loading indicator...');
    const loadingGoose = await mainWindow.waitForSelector('.text-textStandard >> text="goose is working on it…"', 
      { timeout: 10000 });
    expect(await loadingGoose.isVisible()).toBe(true);
    
    // Take screenshot of loading state
    await mainWindow.screenshot({ path: 'test-results/loading-state.png' });
    
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
    await mainWindow.screenshot({ path: 'test-results/chat-response.png' });
  });

  test('verify chat history', async () => {
    console.log('Testing chat history...');
    
    // Find the chat input again
    const chatInput = await mainWindow.waitForSelector('textarea[placeholder*="What can goose help with?"]');
    
    // Test message sending with a specific question
    await chatInput.fill('What is 2+2?');
    
    // Get initial message count
    const initialMessages = await mainWindow.locator('.prose').count();
    
    // Send message
    await chatInput.press('Enter');
    
    // Wait for loading indicator and response using the correct selector
    await mainWindow.waitForSelector('.text-textStandard >> text="goose is working on it…"', { timeout: 10000 });
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
    await mainWindow.screenshot({ path: 'test-results/chat-history.png' });
    
    // Test command history (up arrow)
    await chatInput.press('Control+ArrowUp');
    const inputValue = await chatInput.inputValue();
    expect(inputValue).toBe('What is 2+2?');
  });
});