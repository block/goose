import { Message } from '../api';
import { ChatState } from '../types/chatState';
export function getThinkingMessage(messages: Message[], chatState: ChatState): string | undefined {
  if (chatState === ChatState.Idle) {
    return undefined;
  }

  const lastMessage = messages[messages.length - 1];
  if (!lastMessage || lastMessage.role !== 'assistant') {
    return undefined;
  }

  for (const content of lastMessage.content) {
    if (content.type === 'systemNotification' && content.notificationType === 'thinkingMessage') {
      return content.msg;
    }
  }

  return undefined;
}
