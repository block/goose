import { test } from './fixtures';
import { expect } from '@playwright/test';
import { changeWorkingDirectory, expectChatContainsMessage, expectCostIsNonZero, expectCostIsZero, expectLastChatMessageContains, expectChatMessageCount, expectSessionCount, expectToolCallContainsText, expectToolCallCount, getChatInput, goToChatHistory, goToHome, openSession, sendMessage, startNewChat } from './helpers/test-steps';

test.describe('Goose App', {tag: '@release'}, () => {
  test('goose conversation', async ({ goosePage }) => {
    await goToHome(goosePage);
    await expectCostIsZero(goosePage);

    await sendMessage(goosePage, 'Hello First');
    await sendMessage(goosePage, 'Hello Second');

    await expectCostIsNonZero(goosePage);

    await goToChatHistory(goosePage);
    await expectSessionCount(goosePage, 1);

    await goToHome(goosePage);
    await expectChatMessageCount(goosePage, 0);

    await sendMessage(goosePage, 'tell me a joke');
    await expectChatMessageCount(goosePage, 2);

    await goToChatHistory(goosePage);

    await openSession(goosePage, 2);
    await expectChatContainsMessage(goosePage, 'Hello Second');

    const updatedWorkingDir = await changeWorkingDirectory(goosePage);

    await sendMessage(goosePage, 'what is your working directory? reply with exact path only');
    await expectLastChatMessageContains(goosePage, updatedWorkingDir);
  });

  // recent pr breaks the test and needs to fix
  test.skip('developer tool is called', async ({ goosePage }) => {

    await startNewChat(goosePage);

    await expectToolCallCount(goosePage, 0);

    await sendMessage(goosePage, 'show the number of files in current directory');

    await expectToolCallCount(goosePage, 1);
    await expectToolCallContainsText(goosePage, 1, 'developer extension');
  });

  test('verify chat history', async ({ goosePage }) => {
    await expectChatMessageCount(goosePage, 0);
    await sendMessage(goosePage, 'What is 2+2?');
    await expectChatMessageCount(goosePage, 2);

    const chatInput = getChatInput(goosePage);
    await chatInput.press('Control+ArrowUp');
    await expect(chatInput).toHaveValue('What is 2+2?');
  });
});
