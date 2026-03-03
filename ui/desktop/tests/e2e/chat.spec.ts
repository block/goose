import { test } from './fixtures.electron.packaged';
import { expect } from '@playwright/test';
import { expectChatContainsMessage, expectLastChatMessageContains, expectChatMessageCount, expectSessionCount, goToChatHistory, goToHome, openSession, sendMessage } from './helpers/test-steps';

test.describe('Goose App', {tag: '@release'}, () => {
  test('goose conversation', async ({ goosePage }) => {

    await goosePage.getByTestId('sidebar-chat-button').click();
    await expect(goosePage.getByRole('button', { name: 'New Chat' }).first()).toBeVisible();
    await expect(goosePage.getByTestId('chat-show-all')).toHaveCount(0);

    await goToHome(goosePage);

    const costTrigger = goosePage.getByTestId('bottom-menu-cost-trigger').first();
    const costTooltip = goosePage.getByTestId('bottom-menu-cost-tooltip').first();
    await expect(costTrigger).toContainText('0.0000');
    await costTrigger.hover();
    await expect(costTooltip).toContainText(
      'Input: 0 tokens ($0.000000) | Output: 0 tokens ($0.000000)'
    );

    await sendMessage(goosePage, 'Hello First');

    await sendMessage(goosePage, 'Hello Second');
    
    await costTrigger.hover();
    await expect(costTrigger).not.toContainText('0.0000');
    await expect(costTooltip).not.toContainText(
      'Input: 0 tokens ($0.000000) | Output: 0 tokens ($0.000000)'
    );

    await goToChatHistory(goosePage);
    await expectSessionCount(goosePage, 1);

    await goToHome(goosePage);
    await expectChatMessageCount(goosePage, 0);

    await sendMessage(goosePage, 'Hello from hub');
    await expectChatMessageCount(goosePage, 2);

    await goToChatHistory(goosePage);

    await openSession(goosePage, 2);
    await expectChatContainsMessage(goosePage, 'Hello Second');

    const workingDirButton = goosePage.locator('[data-testid="bottom-menu-dir-switcher"]:visible').first();
    await expect(workingDirButton).toBeVisible();
    const oldWorkingDir = (await workingDirButton.textContent())?.trim() ?? '';
    await workingDirButton.click();
    if (oldWorkingDir) {
      await expect(workingDirButton).not.toContainText(oldWorkingDir);
    }
    const updatedWorkingDir = (await workingDirButton.textContent())?.trim() ?? '';
    expect(updatedWorkingDir.length).toBeGreaterThan(0);

    await sendMessage(goosePage, 'what is your working directory? reply with exact path only');
    await expectLastChatMessageContains(goosePage, updatedWorkingDir);
  });

  test('developer tool is called', async ({ goosePage }) => {

    await goToHome(goosePage);

    const toolCalls = goosePage.locator('.goose-message-tool');
    await expect(toolCalls).toHaveCount(0);

    await sendMessage(goosePage, 'show the number of files in current directory');

    await expect(toolCalls).toHaveCount(1);
    const newestToolCall = toolCalls.first();
    await expect(newestToolCall).toBeVisible();
    const tooltipTrigger = newestToolCall.locator('button.group.w-full span.cursor-pointer').first();
    await expect(tooltipTrigger).toBeVisible();
    await tooltipTrigger.hover();
    await expect(goosePage.getByTestId('tooltip-wrapper-content').first()).toContainText('developer extension');
  });

  test('verify chat history', async ({ goosePage }) => {
    await expectChatMessageCount(goosePage, 0);
    await sendMessage(goosePage, 'What is 2+2?');
    await expectChatMessageCount(goosePage, 2);

    const chatInputForHistory = goosePage.getByTestId('chat-input');
    await chatInputForHistory.press('Control+ArrowUp');
    await expect(chatInputForHistory).toHaveValue('What is 2+2?');
  });
});
