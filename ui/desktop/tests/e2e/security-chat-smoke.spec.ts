import { test, expect } from './fixtures';

test('security preview chat returns a real Token Plan response', async ({ goosePage }) => {
  await goosePage.waitForSelector('[data-testid="chat-input"]', { timeout: 30000 });

  const marker = `SECURITY_GOOSE_PONG_${Date.now()}`;
  const chatInput = goosePage.locator('[data-testid="chat-input"]');
  await chatInput.fill(`Reply with exactly: ${marker}`);
  await chatInput.press('Enter');

  await goosePage.waitForSelector('[data-testid="loading-indicator"]', {
    state: 'visible',
    timeout: 10000,
  });
  await goosePage.waitForSelector('[data-testid="loading-indicator"]', {
    state: 'hidden',
    timeout: 120000,
  });

  await expect(goosePage.locator('[data-testid="message-container"]').last()).toContainText(marker, {
    timeout: 30000,
  });
  await expect(goosePage.getByText(/Provider not set/i)).toHaveCount(0);
  await expect(goosePage.getByText(/Authentication failed/i)).toHaveCount(0);
  await expect(goosePage.getByText(/gpt-5\.3-codex/i)).toHaveCount(0);
});
