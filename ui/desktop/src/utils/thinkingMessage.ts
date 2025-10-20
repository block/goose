import { Message } from '../api';
import { ChatState } from '../types/chatState';
export function getThinkingMessage(messages: Message[], chatState: ChatState): string | undefined {
  // Only look for thinking messages when we're in a loading state
  if (chatState === ChatState.Idle) {
    return undefined;
  }

  // Check the last message for a system notification
  const lastMessage = messages[messages.length - 1];
  if (!lastMessage || lastMessage.role !== 'assistant') {
    return undefined;
  }

  // Look for thinkingMessage systemNotification content only
  for (const content of lastMessage.content) {
    if (content.type === 'systemNotification' && content.notificationType === 'thinkingMessage') {
      return content.msg;
    }
  }

  return undefined;
}
