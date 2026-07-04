import type { ToolCall, ToolCallUpdate } from '@agentclientprotocol/sdk';
import type { TokenState } from '../../types/chat';
import type { Message, NotificationEvent } from '../../types/message';

export type AcpChatStateChange =
  | { type: 'messages'; messages: Message[] }
  | { type: 'messageUpserted'; index: number; message: Message }
  | { type: 'tokenState'; tokenState: Partial<TokenState> }
  | {
      type: 'sessionInfo';
      name?: string;
      activeRunId?: string | null;
    }
  | { type: 'localSteerConfirmed'; messageId: string }
  | { type: 'notification'; notification: NotificationEvent };

export interface AdapterState {
  messages: Message[];
  localSteerTextByMessageId: Map<string, string>;
}

export interface GoslingMessageMeta {
  messageId?: string;
  created?: number;
  steer?: boolean;
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

export function messageUpserted(
  state: AdapterState,
  message: Message,
  index = state.messages.indexOf(message)
): Extract<AcpChatStateChange, { type: 'messageUpserted' }> {
  return {
    type: 'messageUpserted',
    index,
    message: cloneMessage(message),
  };
}

export function cloneMessage(message: Message): Message {
  return {
    ...message,
    content: message.content.map((content) => ({ ...content })),
    metadata: { ...message.metadata },
  };
}

export function getGoslingMessageMeta(update: { _meta?: unknown }): GoslingMessageMeta {
  if (!isRecord(update._meta)) {
    return {};
  }

  const gosling = update._meta.gosling;
  if (!isRecord(gosling)) {
    return {};
  }

  return {
    created: typeof gosling.created === 'number' ? gosling.created : undefined,
    messageId: typeof gosling.messageId === 'string' ? gosling.messageId : undefined,
    steer: gosling.steer === true ? true : undefined,
  };
}

export function getGoslingActiveRunId(update: { _meta?: unknown }): string | null | undefined {
  if (!isRecord(update._meta)) {
    return undefined;
  }

  const gosling = update._meta.gosling;
  if (!isRecord(gosling) || !('activeRunId' in gosling)) {
    return undefined;
  }

  return typeof gosling.activeRunId === 'string' || gosling.activeRunId === null
    ? gosling.activeRunId
    : undefined;
}

export function rawInputToArguments(rawInput: unknown): Record<string, unknown> {
  return isRecord(rawInput) ? rawInput : {};
}

export function toolIdentity(update: ToolCall | ToolCallUpdate): ToolIdentity {
  if (!isRecord(update._meta)) {
    return {};
  }

  const gosling = update._meta.gosling;
  if (!isRecord(gosling) || !isRecord(gosling.toolCall)) {
    return {};
  }

  return {
    toolName: typeof gosling.toolCall.toolName === 'string' ? gosling.toolCall.toolName : undefined,
    extensionName:
      typeof gosling.toolCall.extensionName === 'string' ? gosling.toolCall.extensionName : undefined,
  };
}

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}
