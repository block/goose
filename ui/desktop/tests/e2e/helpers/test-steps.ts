import { expect, Page } from '@playwright/test';
import { waitForLoadingDone } from './video';

export const LLM_TIMEOUT = 30000;

export const getChatInput = (page: Page) =>
  page.locator('[data-testid="chat-input"]:visible').first();

export const sendMessage = async (page: Page, text: string) => {
  const chatInput = getChatInput(page);
  await chatInput.fill(text);
  await chatInput.press('Enter');
  await waitForLoadingDone(page, LLM_TIMEOUT);
};

export const expectChatMessageCount = async (page: Page, count: number) => {
  await expect(page.locator('[data-testid="message-container"]:visible')).toHaveCount(count);
};

export const expectChatContainsMessage = async (page: Page, text: string) => {
  await expect(
    page.locator('[data-testid="message-container"]:visible').filter({ hasText: text }).first()
  ).toBeVisible();
};

export const expectLastChatMessageContains = async (page: Page, text: string | RegExp) => {
  await expect(page.locator('[data-testid="message-container"]:visible').last()).toContainText(text);
};

export const getToolCalls = (page: Page) =>
  page.locator('.goose-message-tool');

export const expectToolCallCount = async (page: Page, count: number) => {
  await expect(getToolCalls(page)).toHaveCount(count);
};

export const expectToolCallContainsText = async (page: Page, position: number, expectedText: string | RegExp) => {
  const toolCall = getToolCalls(page).nth(position - 1);
  await expect(toolCall).toBeVisible();
  const tooltipTrigger = toolCall.locator('button.group.w-full span.cursor-pointer').first();
  await expect(tooltipTrigger).toBeVisible();
  await tooltipTrigger.hover();
  await expect(page.getByTestId('tooltip-wrapper-content').first()).toContainText(expectedText);
};

export const getLastAssistantMessageText = async (page: Page) => {
  return (await page.locator('.goose-message').last().textContent()) ?? '';
};

export const goToHome = async (page: Page) => {
  await page.getByTestId('sidebar-home-button').click();
};

export const expectSessionCount = async (page: Page, count: number) => {
  await expect(page.getByTestId('session-history-card')).toHaveCount(count);
};

export const clickSidebarChat = async (page: Page) => {
  await page.getByTestId('sidebar-chat-button').click();
};

export const goToChatHistory = async (page: Page) => {
  const showAll = page.getByTestId('chat-show-all').first();
  if (!(await showAll.isVisible().catch(() => false))) {
    await clickSidebarChat(page);
  }
  await expect(showAll).toBeVisible();
  await showAll.click();
  await expect(page.getByRole('heading', { name: 'Chat history' })).toBeVisible();
};

export const openSession = async (page: Page, position: number) => {
  await page.getByTestId('session-history-card').nth(position - 1).click();
};

export const startNewChat = async (page: Page) => {
  const newChatButton = page.getByRole('button', { name: 'New Chat' }).first();
  if (!(await newChatButton.isVisible().catch(() => false))) {
    await clickSidebarChat(page);
  }
  await newChatButton.click();
};

export const goToExtensions = async (page: Page) => {
  await page.getByTestId('sidebar-extensions-button').click();
};

export const expectExtensionIsEnabled = async (page: Page, extensionId: string) => {
  await expect(page.locator(`#extension-${extensionId}`)).toBeVisible();
  await expect(
    page.locator(`#extension-${extensionId} button[role="switch"][data-state="checked"]`)
  ).toBeVisible();
};

export const createCustomExtension = async (page: Page, opts: { name: string; description: string; command: string }) => {
  await page.getByRole('button', { name: 'Add custom extension' }).click();
  await page.getByPlaceholder('Enter extension name...').fill(opts.name);
  await page.getByPlaceholder('Optional description...').fill(opts.description);
  await page.getByPlaceholder('e.g. npx -y @modelcontextprotocol/my-extension <filepath>').fill(opts.command);
  await page.getByTestId('extension-submit-btn').click();
};

export const expectCostIsZero = async (page: Page) => {
  const costTrigger = page.getByTestId('bottom-menu-cost-trigger').first();
  const costTooltip = page.getByTestId('bottom-menu-cost-tooltip').first();
  await expect(costTrigger).toContainText('0.0000');
  await costTrigger.hover();
  await expect(costTooltip).toContainText(
    'Input: 0 tokens ($0.000000) | Output: 0 tokens ($0.000000)'
  );
};

export const expectCostIsNonZero = async (page: Page) => {
  const costTrigger = page.getByTestId('bottom-menu-cost-trigger').first();
  const costTooltip = page.getByTestId('bottom-menu-cost-tooltip').first();
  await costTrigger.hover();
  await expect(costTrigger).not.toContainText('0.0000');
  await expect(costTooltip).not.toContainText(
    'Input: 0 tokens ($0.000000) | Output: 0 tokens ($0.000000)'
  );
};

export const changeWorkingDirectory = async (page: Page) => {
  const workingDirButton = page.locator('[data-testid="bottom-menu-dir-switcher"]:visible').first();
  await expect(workingDirButton).toBeVisible();
  const oldWorkingDir = (await workingDirButton.textContent())?.trim() ?? '';
  await workingDirButton.click();
  if (oldWorkingDir) {
    await expect(workingDirButton).not.toContainText(oldWorkingDir);
  }
  const updatedWorkingDir = (await workingDirButton.textContent())?.trim() ?? '';
  expect(updatedWorkingDir.length).toBeGreaterThan(0);
  return updatedWorkingDir;
};

export const openSettingsAppTab = async (page: Page) => {
  await page.getByTestId('sidebar-settings-button').click();
  await page.getByTestId('settings-app-tab').click();
};
