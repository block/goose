import { test, expect, waitForLoadingDone } from './fixtures.electron.packaged';

const LLM_TIMEOUT = 30000;

test.describe('Goose App', () => {
  test('goose conversation', async ({ goosePage }) => {
    const mainWindow = goosePage;

    await mainWindow.getByTestId('sidebar-chat-button').click();
    await expect(mainWindow.getByRole('button', { name: 'New Chat' }).first()).toBeVisible();
    await expect(mainWindow.getByTestId('chat-show-all')).toHaveCount(0);

    await mainWindow.getByTestId('sidebar-home-button').click();
    const chatInput = mainWindow.locator('[data-testid="chat-input"]:visible').first();
    await expect(chatInput).toBeVisible();

    const costTrigger = mainWindow.getByTestId('bottom-menu-cost-trigger').first();
    const costTooltip = mainWindow.getByTestId('bottom-menu-cost-tooltip').first();
    await expect(costTrigger).toContainText('0.0000');
    await costTrigger.hover();
    await expect(costTooltip).toContainText(
      'Input: 0 tokens ($0.000000) | Output: 0 tokens ($0.000000)'
    );

    await chatInput.fill('Hello First');
    await chatInput.press('Enter');

    await waitForLoadingDone(mainWindow, LLM_TIMEOUT);
    await expect(mainWindow.locator('[data-testid="message-container"]:visible').last()).toBeVisible();

    await chatInput.fill('Hello First');
    await chatInput.press('Enter');
    await waitForLoadingDone(mainWindow, LLM_TIMEOUT);
    
    await costTrigger.hover();
    await expect(costTrigger).not.toContainText('0.0000');
    await expect(costTooltip).not.toContainText(
      'Input: 0 tokens ($0.000000) | Output: 0 tokens ($0.000000)'
    );

    const showAllAfterChat = mainWindow.getByTestId('chat-show-all').first();
    if (!(await showAllAfterChat.isVisible().catch(() => false))) {
      await mainWindow.getByTestId('sidebar-chat-button').click();
    }
    await expect(showAllAfterChat).toBeVisible();
    await showAllAfterChat.click();
    await expect(mainWindow.getByRole('heading', { name: 'Chat history' })).toBeVisible();
    const historyCards = mainWindow.getByTestId('session-history-card');
    const historyCountAfterFirstConversation = await historyCards.count();
    expect(historyCountAfterFirstConversation).toBeGreaterThanOrEqual(1);

    await mainWindow.getByTestId('sidebar-home-button').click();
    const hubChatInput = mainWindow.locator('[data-testid="chat-input"]:visible').first();
    await expect(hubChatInput).toBeVisible();
    await expect(mainWindow.locator('[data-testid="message-container"]:visible')).toHaveCount(0);

    await hubChatInput.fill('Hello from hub');
    await hubChatInput.press('Enter');
    await waitForLoadingDone(mainWindow, LLM_TIMEOUT);
    await expect(mainWindow.locator('[data-testid="message-container"]:visible')).toHaveCount(2);

    const showAllAfterHubConversation = mainWindow.getByTestId('chat-show-all').first();
    if (!(await showAllAfterHubConversation.isVisible().catch(() => false))) {
      await mainWindow.getByTestId('sidebar-chat-button').click();
    }
    await expect(showAllAfterHubConversation).toBeVisible();
    await showAllAfterHubConversation.click();
    await expect(mainWindow.getByRole('heading', { name: 'Chat history' })).toBeVisible();

    const originalSessionCard = mainWindow.getByTestId('session-history-card').nth(1);
    await originalSessionCard.click();
    const resumedChatInput = mainWindow.locator('[data-testid="chat-input"]:visible').first();
    await expect(resumedChatInput).toBeVisible();
    await expect(
      mainWindow
        .locator('[data-testid="message-container"]:visible')
        .filter({ hasText: 'Hello First' })
        .first()
    ).toBeVisible();

    const workingDirButton = mainWindow.locator('[data-testid="bottom-menu-dir-switcher"]:visible').first();
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
    await waitForLoadingDone(mainWindow, LLM_TIMEOUT);
    await expect(mainWindow.locator('[data-testid="message-container"]:visible').last()).toContainText(
      updatedWorkingDir
    );
  });

  test('developer tool is called', async ({ goosePage }) => {
    const mainWindow = goosePage;

    await mainWindow.getByTestId('sidebar-home-button').click();
    const chatInput = mainWindow.locator('[data-testid="chat-input"]:visible').first();
    await expect(chatInput).toBeVisible();

    const toolCalls = mainWindow.locator('.goose-message-tool');
    await expect(toolCalls).toHaveCount(0);

    await chatInput.fill('show the number of files in current directory');
    await chatInput.press('Enter');
    await waitForLoadingDone(mainWindow, LLM_TIMEOUT);

    await expect(toolCalls).toHaveCount(1);
    const newestToolCall = toolCalls.first();
    await expect(newestToolCall).toBeVisible();
    const tooltipTrigger = newestToolCall.locator('button.group.w-full span.cursor-pointer').first();
    await expect(tooltipTrigger).toBeVisible();
    await tooltipTrigger.hover();
    await expect(mainWindow.getByTestId('tooltip-wrapper-content').first()).toContainText('developer extension');
  });
});
