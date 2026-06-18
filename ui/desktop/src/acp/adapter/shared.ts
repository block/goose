import type { ToolCall, ToolCallUpdate } from '@agentclientprotocol/sdk';
import type { Message, TokenState } from '../../api';
import type { NotificationEvent } from '../../types/message';

export type AcpChatStateChange =
  | { type: 'messages'; messages: Message[] }
  | { type: 'tokenState'; tokenState: Partial<TokenState> }
  | { type: 'sessionInfo'; name?: string }
  | { type: 'notification'; notification: NotificationEvent };

export interface AdapterState {
  messages: Message[];
}

export interface GooseMessageMeta {
  messageId?: string;
  created?: number;
}

export interface ToolIdentity {
  toolName?: string;
  extensionName?: string;
}

export const DEFAULT_VISIBLE_MESSAGE_METADATA: Message['metadata'] = {
  userVisible: true,
  agentVisible: true,
};

export function messagesChange(state: AdapterState): AcpChatStateChange[] {
  return [{ type: 'messages', messages: state.messages.map(cloneMessage) }];
}

export function cloneMessage(message: Message): Message {
  return {
    ...message,
    content: message.content.map((content) => ({ ...content })),
    metadata: { ...message.metadata },
  };
}

export function getGooseMessageMeta(update: { _meta?: unknown }): GooseMessageMeta {
  if (!isRecord(update._meta)) {
    return {};
  }

  const goose = update._meta.goose;
  if (!isRecord(goose)) {
    return {};
  }

  return {
    created: typeof goose.created === 'number' ? goose.created : undefined,
    messageId: typeof goose.messageId === 'string' ? goose.messageId : undefined,
  };
}

export function rawInputToArguments(rawInput: unknown): Record<string, unknown> {
  return isRecord(rawInput) ? rawInput : {};
}

export function toolIdentity(update: ToolCall | ToolCallUpdate): ToolIdentity {
  if (!isRecord(update._meta)) {
    return {};
  }

  const goose = update._meta.goose;
  if (!isRecord(goose) || !isRecord(goose.toolCall)) {
    return {};
  }

  return {
    toolName: typeof goose.toolCall.toolName === 'string' ? goose.toolCall.toolName : undefined,
    extensionName:
      typeof goose.toolCall.extensionName === 'string' ? goose.toolCall.extensionName : undefined,
  };
}

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}
