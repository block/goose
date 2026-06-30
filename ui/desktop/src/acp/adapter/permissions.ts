import type { RequestPermissionRequest } from '@agentclientprotocol/sdk';
import {
  type AcpChatStateChange,
  type AdapterState,
  DEFAULT_VISIBLE_MESSAGE_METADATA,
  messagesChange,
  rawInputToArguments,
  toolIdentity,
} from './shared';

export function applyPermissionRequest(
  state: AdapterState,
  request: RequestPermissionRequest
): AcpChatStateChange[] {
  const toolCallId = request.toolCall.toolCallId;
  const existing = state.messages.some((message) =>
    message.content.some(
      (content) =>
        content.type === 'actionRequired' &&
        content.data.actionType === 'toolConfirmation' &&
        content.data.id === toolCallId
    )
  );
  if (existing) {
    return messagesChange(state);
  }

  const identity = toolIdentity(request.toolCall);
  const prompt = permissionPrompt(request);

  state.messages.push({
    id: `acp_permission_${toolCallId}`,
    role: 'assistant',
    created: Math.floor(Date.now() / 1000),
    content: [
      {
        type: 'actionRequired',
        data: {
          actionType: 'toolConfirmation',
          id: toolCallId,
          toolName: identity.toolName ?? request.toolCall.title ?? toolCallId,
          arguments: rawInputToArguments(request.toolCall.rawInput),
          ...(prompt ? { prompt } : {}),
        },
      },
    ],
    metadata: { ...DEFAULT_VISIBLE_MESSAGE_METADATA },
  });

  return messagesChange(state);
}

function permissionPrompt(request: RequestPermissionRequest): string | undefined {
  for (const content of request.toolCall.content ?? []) {
    if (content.type === 'content' && content.content.type === 'text') {
      return content.content.text;
    }
  }

  return undefined;
}
