import type { ContentBlock, PromptResponse } from '@agentclientprotocol/sdk';
import type { Message } from '../api';
import { getAcpClient } from './acpConnection';

export async function acpPromptSession(
  sessionId: string,
  message: Message
): Promise<PromptResponse> {
  const client = await getAcpClient();
  return client.prompt({
    sessionId,
    prompt: messageToAcpPromptContent(message),
  });
}

export async function acpCancelPrompt(sessionId: string): Promise<void> {
  const client = await getAcpClient();
  await client.cancel({ sessionId });
}

export function messageToAcpPromptContent(message: Message): ContentBlock[] {
  const prompt: ContentBlock[] = [];

  for (const content of message.content) {
    switch (content.type) {
      case 'text':
        if (content.text.trim()) {
          prompt.push({
            type: 'text',
            text: content.text,
          });
        }
        break;
      case 'image':
        prompt.push({
          type: 'image',
          data: content.data,
          mimeType: content.mimeType,
        });
        break;
    }
  }

  return prompt;
}
