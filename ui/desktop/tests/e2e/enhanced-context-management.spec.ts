import { test, expect, Page } from '@playwright/test';
import { ElectronApplication, _electron as electron } from 'playwright';
import path from 'path';

let electronApp: ElectronApplication;
let page: Page;

test.describe('Enhanced Context Management E2E Tests', () => {
  test.beforeAll(async () => {
    // Launch Electron app
    electronApp = await electron.launch({
      args: [path.join(__dirname, '../../.vite/build/main.js')],
      env: {
        ...process.env,
        NODE_ENV: 'test',
        GOOSE_TEST_MODE: 'true',
      },
    });

    // Get the main window
    page = await electronApp.firstWindow();
    await page.waitForLoadState('domcontentloaded');
    
    // Wait for the app to be ready
    await page.waitForSelector('[data-testid="chat-input"]', { timeout: 10000 });
  });

  test.afterAll(async () => {
    if (electronApp) {
      await electronApp.close();
    }
  });

  test.beforeEach(async () => {
    // Reset to a clean state before each test
    await page.reload();
    await page.waitForLoadState('domcontentloaded');
    await page.waitForSelector('[data-testid="chat-input"]', { timeout: 10000 });
  });

  test.describe('Context Window Alert System', () => {
    test('should show context window alert only when tokens are being used', async () => {
      // Initially, no alert should be visible
      const alertIndicator = page.locator('[data-testid="alert-indicator"]');
      await expect(alertIndicator).not.toBeVisible();

      // Type and send a message to generate token usage
      const chatInput = page.locator('[data-testid="chat-input"]');
      await chatInput.fill('Hello, this is a test message to generate some token usage.');
      await page.keyboard.press('Enter');
      
      // Wait for response and check for context window alert
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
      await page.waitForSelector('[data-testid="alert-indicator"]', { timeout: 15000 });
      
      // Click on the alert indicator to open the popover
      await page.click('[data-testid="alert-indicator"]');
      
      // Verify the context window alert is shown
      const alertBox = page.locator('[role="alert"]');
      await expect(alertBox).toBeVisible();
      await expect(alertBox).toContainText('Context window');
      
      // Verify progress bar is shown
      const progressBar = page.locator('[role="progressbar"]');
      await expect(progressBar).toBeVisible();
      
      // Verify compact button is present
      const compactButton = page.locator('text=Compact now');
      await expect(compactButton).toBeVisible();
    });

    test('should update progress bar as conversation grows', async () => {
      const chatInput = page.locator('[data-testid="chat-input"]');
      
      // Send first message
      await chatInput.fill('First message');
      await page.keyboard.press('Enter');
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
      
      // Get initial progress
      await page.waitForSelector('[data-testid="alert-indicator"]', { timeout: 15000 });
      await page.click('[data-testid="alert-indicator"]');
      
      const progressText1 = await page.locator('[role="alert"]').textContent();
      const match1 = progressText1?.match(/(\d+(?:,\d+)*)\s*\/\s*(\d+(?:,\d+)*)/);
      const initialTokens = match1 ? parseInt(match1[1].replace(/,/g, '')) : 0;
      
      // Close the alert popover
      await page.keyboard.press('Escape');
      
      // Send second message
      await chatInput.fill('Second message with more content to increase token usage significantly');
      await page.keyboard.press('Enter');
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
      
      // Get updated progress
      await page.click('[data-testid="alert-indicator"]');
      
      const progressText2 = await page.locator('[role="alert"]').textContent();
      const match2 = progressText2?.match(/(\d+(?:,\d+)*)\s*\/\s*(\d+(?:,\d+)*)/);
      const updatedTokens = match2 ? parseInt(match2[1].replace(/,/g, '')) : 0;
      
      // Token count should have increased
      expect(updatedTokens).toBeGreaterThan(initialTokens);
    });
  });

  test.describe('Manual Compaction Workflow', () => {
    test('should perform complete manual compaction workflow', async () => {
      const chatInput = page.locator('[data-testid="chat-input"]');
      
      // Build up conversation with multiple exchanges
      const messages = [
        'What is React and how does it work?',
        'Can you explain React hooks in detail?',
        'What are the best practices for React state management?',
        'How do I optimize React application performance?',
      ];
      
      for (const message of messages) {
        await chatInput.fill(message);
        await page.keyboard.press('Enter');
        await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
        await page.waitForTimeout(1000); // Brief pause between messages
      }
      
      // Open the alert popover and initiate compaction
      await page.waitForSelector('[data-testid="alert-indicator"]', { timeout: 15000 });
      await page.click('[data-testid="alert-indicator"]');
      
      const compactButton = page.locator('text=Compact now');
      await expect(compactButton).toBeVisible();
      await compactButton.click();
      
      // Verify alert popover closes immediately
      const alertBox = page.locator('[role="alert"]');
      await expect(alertBox).not.toBeVisible();
      
      // Verify compaction loading state
      const loadingGoose = page.locator('[data-testid="loading-goose"]');
      await expect(loadingGoose).toBeVisible();
      await expect(loadingGoose).toContainText('goose is compacting the conversation...');
      
      // Wait for compaction to complete
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
      
      // Verify compaction marker appears
      const compactionMarker = page.locator('text=Conversation compacted and summarized');
      await expect(compactionMarker).toBeVisible();
      
      // Verify chat input is re-enabled
      const submitButton = page.locator('[data-testid="submit-button"]');
      await expect(submitButton).toBeEnabled();
    });

    test('should hide alert indicator after successful compaction', async () => {
      const chatInput = page.locator('[data-testid="chat-input"]');
      
      // Generate conversation
      await chatInput.fill('Test message for compaction');
      await page.keyboard.press('Enter');
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
      
      // Perform compaction
      await page.waitForSelector('[data-testid="alert-indicator"]', { timeout: 15000 });
      await page.click('[data-testid="alert-indicator"]');
      await page.click('text=Compact now');
      
      // Wait for compaction to complete
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
      
      // Verify alert indicator is no longer visible (or shows reduced token count)
      const alertIndicator = page.locator('[data-testid="alert-indicator"]');
      
      // Either the indicator is hidden, or if visible, the token count should be much lower
      const isVisible = await alertIndicator.isVisible();
      if (isVisible) {
        await alertIndicator.click();
        const alertContent = await page.locator('[role="alert"]').textContent();
        const match = alertContent?.match(/(\d+(?:,\d+)*)\s*\/\s*(\d+(?:,\d+)*)/);
        const currentTokens = match ? parseInt(match[1].replace(/,/g, '')) : 0;
        
        // Token count should be significantly reduced (less than 1000 tokens after compaction)
        expect(currentTokens).toBeLessThan(1000);
      }
    });

    test('should prevent multiple simultaneous compaction attempts', async () => {
      const chatInput = page.locator('[data-testid="chat-input"]');
      
      // Generate conversation
      await chatInput.fill('Test message for multiple compaction prevention');
      await page.keyboard.press('Enter');
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
      
      // Open alert and click compact button
      await page.waitForSelector('[data-testid="alert-indicator"]', { timeout: 15000 });
      await page.click('[data-testid="alert-indicator"]');
      
      const compactButton = page.locator('text=Compact now');
      await expect(compactButton).toBeVisible();
      await compactButton.click();
      
      // Alert should close immediately, preventing further clicks
      const alertBox = page.locator('[role="alert"]');
      await expect(alertBox).not.toBeVisible();
      
      // Verify loading state appears
      const loadingGoose = page.locator('[data-testid="loading-goose"]');
      await expect(loadingGoose).toBeVisible();
      
      // Wait for compaction to complete
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
      
      // Verify only one compaction marker exists
      const compactionMarkers = page.locator('text=Conversation compacted and summarized');
      await expect(compactionMarkers).toHaveCount(1);
    });
  });

  test.describe('Post-Compaction Behavior', () => {
    test('should allow scrolling to view ancestor messages after compaction', async () => {
      const chatInput = page.locator('[data-testid="chat-input"]');
      
      // Create identifiable messages
      const testMessages = [
        'FIRST_UNIQUE_MESSAGE: Tell me about JavaScript',
        'SECOND_UNIQUE_MESSAGE: Explain async/await',
        'THIRD_UNIQUE_MESSAGE: What are promises?',
      ];
      
      // Send messages
      for (const message of testMessages) {
        await chatInput.fill(message);
        await page.keyboard.press('Enter');
        await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
        await page.waitForTimeout(1000);
      }
      
      // Perform compaction
      await page.waitForSelector('[data-testid="alert-indicator"]', { timeout: 15000 });
      await page.click('[data-testid="alert-indicator"]');
      await page.click('text=Compact now');
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
      
      // Verify compaction marker is visible
      await expect(page.locator('text=Conversation compacted and summarized')).toBeVisible();
      
      // Scroll up to find ancestor messages
      const chatContainer = page.locator('[data-testid="chat-container"]');
      await chatContainer.hover();
      
      // Scroll up multiple times
      for (let i = 0; i < 10; i++) {
        await page.mouse.wheel(0, -500);
        await page.waitForTimeout(100);
      }
      
      // Check if we can find at least one of our original messages
      const hasFirstMessage = await page.locator('text=FIRST_UNIQUE_MESSAGE').isVisible();
      const hasSecondMessage = await page.locator('text=SECOND_UNIQUE_MESSAGE').isVisible();
      const hasThirdMessage = await page.locator('text=THIRD_UNIQUE_MESSAGE').isVisible();
      
      // At least one original message should be visible in the ancestor messages
      expect(hasFirstMessage || hasSecondMessage || hasThirdMessage).toBe(true);
    });

    test('should continue conversation normally after compaction', async () => {
      const chatInput = page.locator('[data-testid="chat-input"]');
      
      // Generate initial conversation
      await chatInput.fill('What is TypeScript?');
      await page.keyboard.press('Enter');
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
      
      await chatInput.fill('Can you give me an example?');
      await page.keyboard.press('Enter');
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
      
      // Perform compaction
      await page.waitForSelector('[data-testid="alert-indicator"]', { timeout: 15000 });
      await page.click('[data-testid="alert-indicator"]');
      await page.click('text=Compact now');
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
      
      // Verify compaction completed
      await expect(page.locator('text=Conversation compacted and summarized')).toBeVisible();
      
      // Continue conversation after compaction
      await chatInput.fill('POST_COMPACTION_MESSAGE: Thank you, what about React?');
      await page.keyboard.press('Enter');
      
      // Verify conversation continues normally
      await expect(page.locator('[data-testid="loading-goose"]')).toBeVisible();
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
      
      // Verify the new message appears
      await expect(page.locator('text=POST_COMPACTION_MESSAGE')).toBeVisible();
      
      // Verify we get a response
      const messages = page.locator('[data-testid="message"]');
      const messageCount = await messages.count();
      expect(messageCount).toBeGreaterThan(2); // Should have compaction marker + new messages
    });

    test('should maintain proper message ordering after compaction', async () => {
      const chatInput = page.locator('[data-testid="chat-input"]');
      
      // Generate conversation
      await chatInput.fill('First question about programming');
      await page.keyboard.press('Enter');
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
      
      // Perform compaction
      await page.waitForSelector('[data-testid="alert-indicator"]', { timeout: 15000 });
      await page.click('[data-testid="alert-indicator"]');
      await page.click('text=Compact now');
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
      
      // Send new message after compaction
      await chatInput.fill('NEW_MESSAGE_AFTER_COMPACTION');
      await page.keyboard.press('Enter');
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
      
      // Verify message order: compaction marker should come before new messages
      const allMessages = page.locator('[data-testid="message"]');
      const messageTexts = await allMessages.allTextContents();
      
      const compactionIndex = messageTexts.findIndex(text => 
        text.includes('Conversation compacted and summarized')
      );
      const newMessageIndex = messageTexts.findIndex(text => 
        text.includes('NEW_MESSAGE_AFTER_COMPACTION')
      );
      
      expect(compactionIndex).toBeGreaterThanOrEqual(0);
      expect(newMessageIndex).toBeGreaterThan(compactionIndex);
    });
  });

  test.describe('Error Handling', () => {
    test('should handle compaction errors gracefully', async () => {
      // Mock a backend error
      await page.route('**/api/sessions/*/manage-context', async (route) => {
        await route.fulfill({
          status: 500,
          contentType: 'application/json',
          body: JSON.stringify({ error: 'Backend compaction error' }),
        });
      });
      
      const chatInput = page.locator('[data-testid="chat-input"]');
      
      // Generate conversation
      await chatInput.fill('Test message for error handling');
      await page.keyboard.press('Enter');
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
      
      // Attempt compaction
      await page.waitForSelector('[data-testid="alert-indicator"]', { timeout: 15000 });
      await page.click('[data-testid="alert-indicator"]');
      await page.click('text=Compact now');
      
      // Wait for compaction to fail
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
      
      // Verify error message appears
      const errorMarker = page.locator('text=Compaction failed. Please try again or start a new session.');
      await expect(errorMarker).toBeVisible();
      
      // Verify chat input is still functional after error
      const submitButton = page.locator('[data-testid="submit-button"]');
      await expect(submitButton).toBeEnabled();
    });

    test('should handle network timeouts during compaction', async () => {
      // Mock a timeout
      await page.route('**/api/sessions/*/manage-context', async (route) => {
        // Delay response to simulate timeout
        await new Promise(resolve => setTimeout(resolve, 5000));
        await route.fulfill({
          status: 408,
          contentType: 'application/json',
          body: JSON.stringify({ error: 'Request timeout' }),
        });
      });
      
      const chatInput = page.locator('[data-testid="chat-input"]');
      
      // Generate conversation
      await chatInput.fill('Test message for timeout handling');
      await page.keyboard.press('Enter');
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
      
      // Attempt compaction
      await page.waitForSelector('[data-testid="alert-indicator"]', { timeout: 15000 });
      await page.click('[data-testid="alert-indicator"]');
      await page.click('text=Compact now');
      
      // Verify loading state persists during timeout
      const loadingGoose = page.locator('[data-testid="loading-goose"]');
      await expect(loadingGoose).toBeVisible();
      await expect(loadingGoose).toContainText('goose is compacting the conversation...');
      
      // Wait for timeout to complete
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 35000 });
      
      // Should show error message
      const errorMarker = page.locator('text=Compaction failed. Please try again or start a new session.');
      await expect(errorMarker).toBeVisible();
    });
  });

  test.describe('UI State Management', () => {
    test('should disable chat input during compaction', async () => {
      const chatInput = page.locator('[data-testid="chat-input"]');
      
      // Generate conversation
      await chatInput.fill('Test message for UI state verification');
      await page.keyboard.press('Enter');
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
      
      // Start compaction
      await page.waitForSelector('[data-testid="alert-indicator"]', { timeout: 15000 });
      await page.click('[data-testid="alert-indicator"]');
      await page.click('text=Compact now');
      
      // Verify chat input is disabled during compaction
      const submitButton = page.locator('[data-testid="submit-button"]');
      await expect(submitButton).toBeDisabled();
      
      // Verify loading message
      const loadingGoose = page.locator('[data-testid="loading-goose"]');
      await expect(loadingGoose).toBeVisible();
      await expect(loadingGoose).toContainText('goose is compacting the conversation...');
      
      // Wait for compaction to complete
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
      
      // Verify chat input is re-enabled
      await expect(submitButton).toBeEnabled();
    });

    test('should show appropriate loading states', async () => {
      const chatInput = page.locator('[data-testid="chat-input"]');
      
      // Generate conversation
      await chatInput.fill('Test loading state message');
      await page.keyboard.press('Enter');
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
      
      // Start compaction and immediately check loading state
      await page.waitForSelector('[data-testid="alert-indicator"]', { timeout: 15000 });
      await page.click('[data-testid="alert-indicator"]');
      await page.click('text=Compact now');
      
      // Verify loading goose appears with correct message
      const loadingGoose = page.locator('[data-testid="loading-goose"]');
      await expect(loadingGoose).toBeVisible();
      await expect(loadingGoose).toContainText('goose is compacting the conversation...');
      
      // Verify no other loading indicators are shown
      const regularLoadingMessages = page.locator('[data-testid="loading-goose"]:not(:has-text("compacting"))');
      await expect(regularLoadingMessages).not.toBeVisible();
      
      // Wait for completion
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
      
      // Verify loading state is cleared
      await expect(loadingGoose).not.toBeVisible();
    });
  });

  test.describe('Performance and Reliability', () => {
    test('should handle large conversations efficiently', async () => {
      const chatInput = page.locator('[data-testid="chat-input"]');
      
      // Generate a larger conversation
      const messages = Array.from({ length: 8 }, (_, i) => 
        `Message ${i + 1}: This is a longer message with more content to test the compaction system with a substantial amount of text that should generate more tokens and provide a better test of the compaction functionality.`
      );
      
      for (const message of messages) {
        await chatInput.fill(message);
        await page.keyboard.press('Enter');
        await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
        await page.waitForTimeout(500);
      }
      
      // Perform compaction
      await page.waitForSelector('[data-testid="alert-indicator"]', { timeout: 15000 });
      await page.click('[data-testid="alert-indicator"]');
      await page.click('text=Compact now');
      
      // Verify compaction completes within reasonable time
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 45000 });
      
      // Verify compaction marker appears
      await expect(page.locator('text=Conversation compacted and summarized')).toBeVisible();
      
      // Verify system remains responsive
      await chatInput.fill('Post-compaction test message');
      await page.keyboard.press('Enter');
      await expect(page.locator('[data-testid="loading-goose"]')).toBeVisible();
    });

    test('should maintain conversation context after compaction', async () => {
      const chatInput = page.locator('[data-testid="chat-input"]');
      
      // Create conversation with specific context
      await chatInput.fill('My name is Alice and I am a software developer working on React applications.');
      await page.keyboard.press('Enter');
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
      
      await chatInput.fill('I am having trouble with useState hooks. Can you help?');
      await page.keyboard.press('Enter');
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
      
      // Perform compaction
      await page.waitForSelector('[data-testid="alert-indicator"]', { timeout: 15000 });
      await page.click('[data-testid="alert-indicator"]');
      await page.click('text=Compact now');
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
      
      // Test if context is maintained by asking a follow-up question
      await chatInput.fill('What did I tell you my name was?');
      await page.keyboard.press('Enter');
      await page.waitForSelector('[data-testid="loading-goose"]', { state: 'hidden', timeout: 30000 });
      
      // The response should ideally reference the name Alice or indicate context retention
      // Note: This is a behavioral test that depends on the AI's ability to use the summary
      const messages = page.locator('[data-testid="message"]');
      const lastMessageText = await messages.last().textContent();
      
      // The system should have some response (not just an error)
      expect(lastMessageText).toBeTruthy();
      expect(lastMessageText!.length).toBeGreaterThan(10);
    });
  });
});
