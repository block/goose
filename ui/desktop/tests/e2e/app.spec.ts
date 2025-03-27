import { test, expect } from '@playwright/test';
import { _electron as electron } from '@playwright/test';
import { join } from 'path';
import { spawn, exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

test.describe('Goose App', () => {
  let electronApp;
  let appProcess;

  test.beforeEach(async () => {
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
  });

  test.afterEach(async () => {
    console.log('Cleaning up...');
    
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
      // Ignore errors if no processes found
      if (!error.message?.includes('no process found')) {
        console.error('Error killing electron processes:', error);
      }
    }

    // Kill any remaining npm processes from start-gui
    try {
      if (process.platform === 'win32') {
        await execAsync('taskkill /F /IM node.exe');
      } else {
        // The || true ensures the command doesn't fail if no processes are found
        await execAsync('pkill -f "start-gui" || true');
      }
    } catch (error) {
      // Only log real errors, not "no matching processes" errors
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
      // Ignore ESRCH errors (process not found)
      if (error.code !== 'ESRCH') {
        console.error('Error killing npm process:', error);
      }
    }
  });

  test('basic app functionality', async () => {
    console.log('Starting test...');
    
    try {
      // Get the first window
      console.log('Getting first window...');
      const window = await electronApp.firstWindow();
      
      // Wait for the window to load
      console.log('Waiting for window load...');
      await window.waitForLoadState('domcontentloaded');
      
      // Take a screenshot at this point
      console.log('Taking first screenshot...');
      await window.screenshot({ path: 'test-results/window-loaded.png' });
      
      // Wait for the provider selection screen with correct casing
      console.log('Waiting for provider selection screen...');
      const heading = await window.waitForSelector('h2:has-text("Choose a Provider")', { timeout: 10000 });
      const headingText = await heading.textContent();
      expect(headingText).toBe('Choose a Provider');
      
      // Take a screenshot of the provider selection screen
      await window.screenshot({ path: 'test-results/provider-selection.png' });
      
      // Get the window title
      console.log('Getting window title...');
      const title = await window.title();
      console.log('Window title:', title);
      
      // Verify we're on the provider selection screen
      const providerText = await window.textContent('h2');
      expect(providerText).toBe('Choose a Provider');
      
      console.log('Test completed successfully');
    } catch (error) {
      console.error('Test failed:', error);
      
      // Try to take an error screenshot
      try {
        const window = await electronApp.firstWindow();
        await window.screenshot({ path: 'test-results/error-state.png' });
      } catch (screenshotError) {
        console.error('Failed to take error screenshot:', screenshotError);
      }
      
      throw error;
    }
  });
});