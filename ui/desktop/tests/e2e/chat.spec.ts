import { test } from './fixtures.electron.packaged';
import { expect } from '@playwright/test';
import { waitForLoadingDone } from './helpers/video';

const LLM_TIMEOUT = 30000;

test.describe('Goose App', {tag: '@release'}, () => {
  test('goose conversation', async ({ goosePage }) => {

    await goosePage.getByTestId('sidebar-chat-button').click();
    await expect(goosePage.getByRole('button', { name: 'New Chat' }).first()).toBeVisible();
    await expect(goosePage.getByTestId('chat-show-all')).toHaveCount(0);

    await goosePage.getByTestId('sidebar-home-button').click();
    const chatInput = goosePage.locator('[data-testid="chat-input"]:visible').first();
    await expect(chatInput).toBeVisible();

    const costTrigger = goosePage.getByTestId('bottom-menu-cost-trigger').first();
    const costTooltip = goosePage.getByTestId('bottom-menu-cost-tooltip').first();
    await expect(costTrigger).toContainText('0.0000');
    await costTrigger.hover();
    await expect(costTooltip).toContainText(
      'Input: 0 tokens ($0.000000) | Output: 0 tokens ($0.000000)'
    );

    await chatInput.fill('Hello First');
    await chatInput.press('Enter');

    await waitForLoadingDone(goosePage, LLM_TIMEOUT);
    await expect(goosePage.locator('[data-testid="message-container"]:visible').last()).toBeVisible();

    await chatInput.fill('Hello First');
    await chatInput.press('Enter');
    await waitForLoadingDone(goosePage, LLM_TIMEOUT);
    
    await costTrigger.hover();
    await expect(costTrigger).not.toContainText('0.0000');
    await expect(costTooltip).not.toContainText(
      'Input: 0 tokens ($0.000000) | Output: 0 tokens ($0.000000)'
    );

    const showAllAfterChat = goosePage.getByTestId('chat-show-all').first();
    if (!(await showAllAfterChat.isVisible().catch(() => false))) {
      await goosePage.getByTestId('sidebar-chat-button').click();
    }
    await expect(showAllAfterChat).toBeVisible();
    await showAllAfterChat.click();
    await expect(goosePage.getByRole('heading', { name: 'Chat history' })).toBeVisible();
    const historyCards = goosePage.getByTestId('session-history-card');
    const historyCountAfterFirstConversation = await historyCards.count();
    expect(historyCountAfterFirstConversation).toBeGreaterThanOrEqual(1);

    await goosePage.getByTestId('sidebar-home-button').click();
    const hubChatInput = goosePage.locator('[data-testid="chat-input"]:visible').first();
    await expect(hubChatInput).toBeVisible();
    await expect(goosePage.locator('[data-testid="message-container"]:visible')).toHaveCount(0);

    await hubChatInput.fill('Hello from hub');
    await hubChatInput.press('Enter');
    await waitForLoadingDone(goosePage, LLM_TIMEOUT);
    await expect(goosePage.locator('[data-testid="message-container"]:visible')).toHaveCount(2);

    const showAllAfterHubConversation = goosePage.getByTestId('chat-show-all').first();
    if (!(await showAllAfterHubConversation.isVisible().catch(() => false))) {
      await goosePage.getByTestId('sidebar-chat-button').click();
    }
    await expect(showAllAfterHubConversation).toBeVisible();
    await showAllAfterHubConversation.click();
    await expect(goosePage.getByRole('heading', { name: 'Chat history' })).toBeVisible();

    const originalSessionCard = goosePage.getByTestId('session-history-card').nth(1);
    await originalSessionCard.click();
    const resumedChatInput = goosePage.locator('[data-testid="chat-input"]:visible').first();
    await expect(resumedChatInput).toBeVisible();
    await expect(
      goosePage
        .locator('[data-testid="message-container"]:visible')
        .filter({ hasText: 'Hello First' })
        .first()
    ).toBeVisible();

    const workingDirButton = goosePage.locator('[data-testid="bottom-menu-dir-switcher"]:visible').first();
    await expect(workingDirButton).toBeVisible();
    const oldWorkingDir = (await workingDirButton.textContent())?.trim() ?? '';
    await workingDirButton.click();
    if (oldWorkingDir) {
      await expect(workingDirButton).not.toContainText(oldWorkingDir);
    }
    const updatedWorkingDir = (await workingDirButton.textContent())?.trim() ?? '';
    expect(updatedWorkingDir.length).toBeGreaterThan(0);

    await resumedChatInput.fill('what is your working directory? reply with exact path only');
    await resumedChatInput.press('Enter');
    await waitForLoadingDone(goosePage, LLM_TIMEOUT);
    await expect(goosePage.locator('[data-testid="message-container"]:visible').last()).toContainText(
      updatedWorkingDir
    );
  });

  test('developer tool is called', async ({ goosePage }) => {

    await goosePage.getByTestId('sidebar-home-button').click();
    const chatInput = goosePage.locator('[data-testid="chat-input"]:visible').first();
    await expect(chatInput).toBeVisible();

    const toolCalls = goosePage.locator('.goose-message-tool');
    await expect(toolCalls).toHaveCount(0);

    await chatInput.fill('show the number of files in current directory');
    await chatInput.press('Enter');
    await waitForLoadingDone(goosePage, LLM_TIMEOUT);

    await expect(toolCalls).toHaveCount(1);
    const newestToolCall = toolCalls.first();
    await expect(newestToolCall).toBeVisible();
    const tooltipTrigger = newestToolCall.locator('button.group.w-full span.cursor-pointer').first();
    await expect(tooltipTrigger).toBeVisible();
    await tooltipTrigger.hover();
    await expect(goosePage.getByTestId('tooltip-wrapper-content').first()).toContainText('developer extension');
  });

  test('verify chat history', async ({ goosePage }) => {
    const chatInput = goosePage.getByTestId('chat-input');
    await chatInput.fill('What is 2+2?');
    await chatInput.press('Enter');

    await waitForLoadingDone(goosePage, LLM_TIMEOUT);

    const response = goosePage.getByTestId('message-container').last();
    const responseText = await response.textContent();
    expect(responseText).toBeTruthy();

    await expect(goosePage.getByTestId('message-container')).toHaveCount(2);

    const chatInputForHistory = goosePage.getByTestId('chat-input');
    await chatInputForHistory.press('Control+ArrowUp');
    await expect(chatInputForHistory).toHaveValue('What is 2+2?');
  });
});
