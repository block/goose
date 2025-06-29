import { test, expect } from '@playwright/test';
import { _electron as electron } from '@playwright/test';
import { join } from 'path';

test.describe('Prompt History Menu', () => {
  let electronApp;
  let mainWindow;

  test.beforeAll(async () => {
    // Launch Electron app
    electronApp = await electron.launch({
      args: ['.vite/build/main.js'],
      cwd: join(__dirname, '../..'),
      env: {
        ...process.env,
        ELECTRON_IS_DEV: '1',
        NODE_ENV: 'development',
      }
    });

    mainWindow = await electronApp.firstWindow();
    await mainWindow.waitForLoadState('domcontentloaded');
    
    // Wait for React app to be ready
    await mainWindow.waitForFunction(() => {
      const root = document.getElementById('root');
      return root && root.children.length > 0;
    });
    
    await mainWindow.waitForTimeout(2000);
  });

  test.afterAll(async () => {
    if (electronApp) {
      await electronApp.close();
    }
  });

  // Helper function to get chat input and check if we're on macOS
  const getChatInput = async () => {
    await mainWindow.waitForSelector('[data-testid="chat-input"]', { timeout: 10000 });
    return {
      chatInput: mainWindow.locator('[data-testid="chat-input"]'),
      isMacOS: process.platform === 'darwin'
    };
  };

  test('should navigate history with Cmd+Up/Down and show correct placeholder', async () => {
    const { chatInput, isMacOS } = await getChatInput();
    
    // Check placeholder shows new keyboard shortcuts
    const placeholder = await chatInput.getAttribute('placeholder');
    expect(placeholder).toContain('⌘↑/⌘↓');
    expect(placeholder).not.toContain('⇧⌘↑/⇧⌘↓');
    
    // Type and submit messages to create history
    await chatInput.fill('test prompt 1');
    await chatInput.press('Enter');
    await mainWindow.waitForTimeout(1000);
    
    await chatInput.fill('test prompt 2');
    await chatInput.press('Enter');
    await mainWindow.waitForTimeout(1000);
    
    // Test history navigation
    await chatInput.click();
    await chatInput.press(isMacOS ? 'Meta+ArrowUp' : 'Control+ArrowUp');
    
    const inputValue = await chatInput.inputValue();
    expect(inputValue).toBeTruthy();
    expect(inputValue.length).toBeGreaterThan(0);
    
    // Test next prompt
    await chatInput.press(isMacOS ? 'Meta+ArrowDown' : 'Control+ArrowDown');
    const newInputValue = await chatInput.inputValue();
    expect(newInputValue).not.toBe(inputValue);
  });

  test('should not navigate history with Shift+Cmd+Arrow keys', async () => {
    const { chatInput, isMacOS } = await getChatInput();
    
    await chatInput.fill('test prompt');
    await chatInput.press('Enter');
    await mainWindow.waitForTimeout(1000);
    
    await chatInput.fill('');
    await chatInput.press(isMacOS ? 'Shift+Meta+ArrowUp' : 'Shift+Control+ArrowUp');
    
    const inputValue = await chatInput.inputValue();
    expect(inputValue).toBe('');
  });
});
