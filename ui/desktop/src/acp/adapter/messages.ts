import type {
  ContentBlock as AcpContentBlock,
  SessionNotification,
} from '@agentclientprotocol/sdk';
import type { ContentBlock, Message } from '../../types/message';
import {
  type AcpChatStateChange,
  type AdapterState,
  DEFAULT_VISIBLE_MESSAGE_METADATA,
  getGooseMessageMeta,
  messagesChange,
} from './shared';

type StreamedContentBlock = Extract<ContentBlock, { type: 'text' | 'image' }>;

export function applyContentChunk(
  state: AdapterState,
  role: Message['role'],
  update: Extract<
    SessionNotification['update'],
    { sessionUpdate: 'user_message_chunk' | 'agent_message_chunk' }
  >
): AcpChatStateChange[] {
  const content = messageContentFromAcpContentBlock(update.content);
  if (!content) {
    return [];
  }

  const gooseMeta = getGooseMessageMeta(update);
  const messageId = update.messageId ?? gooseMeta.messageId;
  const existing = findMessageForChunk(state, role, messageId, gooseMeta.created);

  if (existing) {
    const isOutputLimitFallbackChunk =
      gooseMeta.outputTokenLimitReached === true && gooseMeta.fallbackContent === true;
    const existingMessageHasContent = existing.content.length > 0;
    const shouldSkipFallbackChunk = isOutputLimitFallbackChunk && existingMessageHasContent;

    existing.metadata.outputTokenLimitReached = gooseMeta.outputTokenLimitReached;
    existing.metadata.fallbackContent = shouldSkipFallbackChunk
      ? undefined
      : gooseMeta.fallbackContent;

    if (shouldSkipFallbackChunk) {
      return messagesChangeWithLocalSteerConfirmation(state, existing, gooseMeta.steer);
    }

    const lastContent = existing.content[existing.content.length - 1];
    if (reconcileLocalSteerTextChunk(state, existing, content, gooseMeta.steer)) {
      return messagesChangeWithLocalSteerConfirmation(state, existing, gooseMeta.steer);
    }

    if (lastContent?.type === 'text' && content.type === 'text') {
      replaceMessage(state, existing, {
        ...existing,
        content: [
          ...existing.content.slice(0, -1),
          { ...lastContent, text: lastContent.text + content.text },
        ],
      });
    } else if (content.type === 'image' && hasImageContent(existing, content)) {
      return messagesChangeWithLocalSteerConfirmation(state, existing, gooseMeta.steer);
    } else {
      replaceMessage(state, existing, {
        ...existing,
        content: [...existing.content, content],
      });
    }

    return messagesChangeWithLocalSteerConfirmation(state, existing, gooseMeta.steer);
  } else {
    state.messages.push({
      ...(messageId ? { id: messageId } : {}),
      role,
      created: gooseMeta.created ?? Math.floor(Date.now() / 1000),
      content: [content],
      metadata: {
        ...DEFAULT_VISIBLE_MESSAGE_METADATA,
        ...(gooseMeta.steer ? { steer: true } : {}),
        outputTokenLimitReached: gooseMeta.outputTokenLimitReached,
        fallbackContent: gooseMeta.fallbackContent,
      },
    });
  }

  return messagesChange(state);
}

export function applyThoughtChunk(
  state: AdapterState,
  update: Extract<SessionNotification['update'], { sessionUpdate: 'agent_thought_chunk' }>
): AcpChatStateChange[] {
  if (update.content.type !== 'text') {
    return [];
  }

  const gooseMeta = getGooseMessageMeta(update);
  const messageId = update.messageId ?? gooseMeta.messageId;
  const existing = findMessageForChunk(state, 'assistant', messageId, gooseMeta.created);

  if (existing) {
    const lastContent = existing.content[existing.content.length - 1];
    const nextContent =
      lastContent?.type === 'thinking'
        ? [
            ...existing.content.slice(0, -1),
            { ...lastContent, thinking: lastContent.thinking + update.content.text },
          ]
        : [
            ...existing.content,
            { type: 'thinking' as const, thinking: update.content.text, signature: '' },
          ];
    replaceMessage(state, existing, {
      ...existing,
      metadata: {
        ...existing.metadata,
        outputTokenLimitReached: gooseMeta.outputTokenLimitReached,
      },
      content: nextContent,
    });
  } else {
    state.messages.push({
      ...(messageId ? { id: messageId } : {}),
      role: 'assistant',
      created: gooseMeta.created ?? Math.floor(Date.now() / 1000),
      content: [{ type: 'thinking', thinking: update.content.text, signature: '' }],
      metadata: {
        ...DEFAULT_VISIBLE_MESSAGE_METADATA,
        outputTokenLimitReached: gooseMeta.outputTokenLimitReached,
      },
    });
  }

  return messagesChange(state);
}

function messageContentFromAcpContentBlock(
  content: AcpContentBlock
): StreamedContentBlock | undefined {
  switch (content.type) {
    case 'text':
      return {
        type: 'text',
        text: content.text,
        ...(content._meta ? { _meta: content._meta } : {}),
        ...(content.annotations ? { annotations: content.annotations } : {}),
      };
    case 'image':
      return {
        type: 'image',
        data: content.data,
        mimeType: content.mimeType,
        ...(content._meta ? { _meta: content._meta } : {}),
        ...(content.annotations ? { annotations: content.annotations } : {}),
      };
    default:
      return undefined;
  }
}

export function findMessageForChunk(
  state: AdapterState,
  role: Message['role'],
  messageId: string | undefined,
  created: number | undefined
): Message | undefined {
  if (!messageId) {
    return lastMergeableMessageWithRole(state, role);
  }

  const existing = state.messages.find(
    (message) => message.id === messageId && message.role === role
  );
  if (existing) {
    return existing;
  }

  const pending = lastMergeableMessageWithRole(state, role);
  if (pending && !pending.id) {
    return replaceMessage(state, pending, {
      ...pending,
      id: messageId,
      created: created ?? pending.created,
    });
  }

  return undefined;
}

function lastMergeableMessageWithRole(
  state: AdapterState,
  role: Message['role']
): Message | undefined {
  const lastMessage = state.messages[state.messages.length - 1];
  if (lastMessage?.role !== role || lastMessage.metadata.agentVisible === false) {
    return undefined;
  }
  return lastMessage;
}

function hasImageContent(
  message: Message,
  image: Extract<StreamedContentBlock, { type: 'image' }>
) {
  return message.content.some(
    (content) =>
      content.type === 'image' && content.data === image.data && content.mimeType === image.mimeType
  );
}

function messagesChangeWithLocalSteerConfirmation(
  state: AdapterState,
  message: Message,
  isSteerChunk: boolean | undefined
): AcpChatStateChange[] {
  const changes = messagesChange(state);
  if (isSteerChunk && message.metadata.steer && message.role === 'user' && message.id) {
    changes.push({ type: 'localSteerConfirmed', messageId: message.id });
  }
  return changes;
}

function reconcileLocalSteerTextChunk(
  state: AdapterState,
  message: Message,
  content: StreamedContentBlock,
  isSteerChunk: boolean | undefined
): boolean {
  if (!isSteerChunk || !message.metadata.steer || message.role !== 'user') {
    return false;
  }

  if (
    content.type !== 'text' ||
    message.content.length === 0 ||
    message.content[0].type !== 'text'
  ) {
    return false;
  }

  const text = (message.id ? state.localSteerTextByMessageId.get(message.id) : undefined) ?? '';
  const nextText = text + content.text;
  if (message.id) {
    state.localSteerTextByMessageId.set(message.id, nextText);
  }

  replaceMessage(state, message, {
    ...message,
    content: [{ ...content, text: nextText }, ...message.content.slice(1)],
    metadata: { ...message.metadata, steer: true },
  });
  return true;
}

export function replaceMessage(state: AdapterState, previous: Message, next: Message): Message {
  const index = state.messages.indexOf(previous);
  if (index === -1) {
    state.messages.push(next);
    return next;
  }
  state.messages[index] = next;
  return next;
}
