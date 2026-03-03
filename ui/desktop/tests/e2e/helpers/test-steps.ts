import { expect, Page } from '@playwright/test';
import { waitForLoadingDone } from './video';

export const LLM_TIMEOUT = 30000;

export async function sendMessage(page: Page, text: string): Promise<void> {
  const chatInput = page.locator('[data-testid="chat-input"]:visible').first();
  await chatInput.fill(text);
  await chatInput.press('Enter');
  await waitForLoadingDone(page, LLM_TIMEOUT);
}

export async function expectChatMessageCount(page: Page, count: number): Promise<void> {
  await expect(page.locator('[data-testid="message-container"]:visible')).toHaveCount(count);
}

export async function expectChatContainsMessage(page: Page, text: string): Promise<void> {
  await expect(
    page.locator('[data-testid="message-container"]:visible').filter({ hasText: text }).first()
  ).toBeVisible();
}

export async function expectLastChatMessageContains(page: Page, text: string | RegExp): Promise<void> {
  await expect(page.locator('[data-testid="message-container"]:visible').last()).toContainText(text);
}

export async function goToHome(page: Page): Promise<void> {
  await page.getByTestId('sidebar-home-button').click();
}

export async function expectSessionCount(page: Page, count: number): Promise<void> {
  await expect(page.getByTestId('session-history-card')).toHaveCount(count);
}

export async function goToChatHistory(page: Page): Promise<void> {
  const showAll = page.getByTestId('chat-show-all').first();
  if (!(await showAll.isVisible().catch(() => false))) {
    await page.getByTestId('sidebar-chat-button').click();
  }
  await expect(showAll).toBeVisible();
  await showAll.click();
  await expect(page.getByRole('heading', { name: 'Chat history' })).toBeVisible();
}

export async function openSession(page: Page, position: number): Promise<void> {
  await page.getByTestId('session-history-card').nth(position - 1).click();
}

export async function openSettingsAppTab(page: Page): Promise<void> {
  await page.getByTestId('sidebar-settings-button').click();
  await page.getByTestId('settings-app-tab').click();
}
