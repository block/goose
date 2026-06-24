import type { ContentBlock, PromptResponse } from '@agentclientprotocol/sdk';
import type { SteerSessionRequest_unstable, SteerSessionResponse_unstable } from '@aaif/goose-sdk';
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

export async function acpSteerSession(
  sessionId: string,
  message: Message,
  expectedRunId: string
): Promise<SteerSessionResponse_unstable> {
  const client = await getAcpClient();
  return client.goose.sessionSteer_unstable({
    sessionId,
    expectedRunId,
    prompt: messageToAcpPromptContent(message) as unknown as SteerSessionRequest_unstable['prompt'],
  });
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
