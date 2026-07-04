import type {
  ContentBlock as AcpContentBlock,
  SessionNotification,
} from '@agentclientprotocol/sdk';
import type { ContentBlock, Message } from '../../types/message';
import {
  type AcpChatStateChange,
  type AdapterState,
  DEFAULT_VISIBLE_MESSAGE_METADATA,
  getGoslingMessageMeta,
  messageUpserted,
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

  const goslingMeta = getGoslingMessageMeta(update);
  const messageId = update.messageId ?? goslingMeta.messageId;
  const existing = findMessageForChunk(state, role, messageId, goslingMeta.created);

  if (existing) {
    const lastContent = existing.content[existing.content.length - 1];
    if (reconcileLocalSteerTextChunk(state, existing, content, goslingMeta.steer)) {
      return messagesChangeWithLocalSteerConfirmation(state, existing, goslingMeta.steer);
    }

    if (lastContent?.type === 'text' && content.type === 'text') {
      lastContent.text += content.text;
    } else if (content.type === 'image' && hasImageContent(existing, content)) {
      return messagesChangeWithLocalSteerConfirmation(state, existing, goslingMeta.steer);
    } else {
      existing.content.push(content);
    }

    return messagesChangeWithLocalSteerConfirmation(state, existing, goslingMeta.steer);
  } else {
    state.messages.push({
      ...(messageId ? { id: messageId } : {}),
      role,
      created: goslingMeta.created ?? Math.floor(Date.now() / 1000),
      content: [content],
      metadata: {
        ...DEFAULT_VISIBLE_MESSAGE_METADATA,
        ...(goslingMeta.steer ? { steer: true } : {}),
      },
    });
  }

  return [messageUpserted(state, state.messages[state.messages.length - 1])];
}

export function applyThoughtChunk(
  state: AdapterState,
  update: Extract<SessionNotification['update'], { sessionUpdate: 'agent_thought_chunk' }>
): AcpChatStateChange[] {
  if (update.content.type !== 'text') {
    return [];
  }

  const goslingMeta = getGoslingMessageMeta(update);
  const messageId = update.messageId ?? goslingMeta.messageId;
  const existing = findMessageForChunk(state, 'assistant', messageId, goslingMeta.created);

  if (existing) {
    const lastContent = existing.content[existing.content.length - 1];
    if (lastContent?.type === 'thinking') {
      lastContent.thinking += update.content.text;
    } else {
      existing.content.push({ type: 'thinking', thinking: update.content.text, signature: '' });
    }
  } else {
    state.messages.push({
      ...(messageId ? { id: messageId } : {}),
      role: 'assistant',
      created: goslingMeta.created ?? Math.floor(Date.now() / 1000),
      content: [{ type: 'thinking', thinking: update.content.text, signature: '' }],
      metadata: { ...DEFAULT_VISIBLE_MESSAGE_METADATA },
    });
  }

  return [messageUpserted(state, existing ?? state.messages[state.messages.length - 1])];
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
    pending.id = messageId;
    pending.created = created ?? pending.created;
    return pending;
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
  const changes: AcpChatStateChange[] = [messageUpserted(state, message)];
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

  message.content = [{ ...content, text: nextText }, ...message.content.slice(1)];
  message.metadata = { ...message.metadata, steer: true };
  return true;
}
