import type { GooseSessionNotification_unstable as GooseSessionNotification } from '@aaif/goose-sdk';
import type {
  ContentBlock,
  SessionConfigOption,
  SessionNotification,
  RequestPermissionRequest,
  ToolCall,
  ToolCallUpdate,
} from '@agentclientprotocol/sdk';
import type { Message, MessageContent, TokenState } from '../api';

export type AcpSessionUpdate =
  | { type: 'messages'; messages: Message[] }
  | { type: 'sessionInfo'; name?: string }
  | { type: 'configOptions'; configOptions: SessionConfigOption[] }
  | { type: 'tokenState'; tokenState: Partial<TokenState> };

interface AdapterState {
  messages: Message[];
}

interface AcpReplayMessageMeta {
  created?: number;
  messageId?: string;
}

interface AcpReplayMetaContainer {
  _meta?: unknown;
}

type GooseInteractionUpdate = Extract<
  GooseSessionNotification['update'],
  { sessionUpdate: 'interaction_update' }
>;

const DEFAULT_VISIBLE_MESSAGE_METADATA: Message['metadata'] = {
  userVisible: true,
  agentVisible: true,
};

export interface AcpSessionSnapshot {
  messages: Message[];
}

export interface AcpSessionNotificationAdapter {
  apply(notification: SessionNotification): AcpSessionUpdate[];
  applyGoose(notification: GooseSessionNotification): AcpSessionUpdate[];
  applyPermissionRequest(request: RequestPermissionRequest): AcpSessionUpdate[];
  snapshot(): AcpSessionSnapshot;
}

export function createAcpSessionNotificationAdapter(
  initialMessages: Message[] = []
): AcpSessionNotificationAdapter {
  const state: AdapterState = {
    messages: initialMessages.map(cloneMessage),
  };

  return {
    apply(notification) {
      return applyAcpSessionNotification(state, notification);
    },
    applyGoose(notification) {
      return applyGooseSessionNotification(state, notification);
    },
    applyPermissionRequest(request) {
      return applyPermissionRequest(state, request);
    },
    snapshot() {
      return {
        messages: state.messages.map(cloneMessage),
      };
    },
  };
}

function applyAcpSessionNotification(
  state: AdapterState,
  notification: SessionNotification
): AcpSessionUpdate[] {
  const { update } = notification;

  switch (update.sessionUpdate) {
    case 'user_message_chunk':
      return applyContentChunk(state, 'user', update);

    case 'agent_message_chunk':
      return applyContentChunk(state, 'assistant', update);

    case 'agent_thought_chunk':
      return applyThoughtChunk(state, update);

    case 'tool_call':
      return applyToolCall(state, update);

    case 'tool_call_update':
      return applyToolCallUpdate(state, update);

    case 'session_info_update':
      return [{ type: 'sessionInfo', name: update.title ?? undefined }];

    case 'config_option_update':
      return [{ type: 'configOptions', configOptions: update.configOptions }];

    default:
      return [];
  }
}

function applyGooseSessionNotification(
  state: AdapterState,
  notification: GooseSessionNotification
): AcpSessionUpdate[] {
  const { update } = notification;

  switch (update.sessionUpdate) {
    case 'usage_update':
      return [
        {
          type: 'tokenState',
          tokenState: {
            totalTokens: update.used,
            accumulatedInputTokens: update.accumulatedInputTokens,
            accumulatedOutputTokens: update.accumulatedOutputTokens,
            accumulatedTotalTokens: update.accumulatedInputTokens + update.accumulatedOutputTokens,
            accumulatedCost: update.accumulatedCost,
          },
        },
      ];
    case 'status_message':
      return applyStatusMessage(state, notification.sessionId, update);
    case 'interaction_update':
      return applyInteractionUpdate(state, notification.sessionId, update);
  }
}

function applyStatusMessage(
  state: AdapterState,
  sessionId: string,
  update: Extract<GooseSessionNotification['update'], { sessionUpdate: 'status_message' }>
): AcpSessionUpdate[] {
  const message = messageFromStatusMessage(sessionId, update);
  state.messages.push(message);
  return [{ type: 'messages', messages: state.messages.map(cloneMessage) }];
}

function messageFromStatusMessage(
  sessionId: string,
  update: Extract<GooseSessionNotification['update'], { sessionUpdate: 'status_message' }>
): Message {
  const { status } = update;
  const baseMessage = {
    id: `acp_status_${sessionId}_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`,
    role: 'assistant' as const,
    created: Math.floor(Date.now() / 1000),
    metadata: { userVisible: true, agentVisible: false },
  };

  switch (status.type) {
    case 'notice':
      return {
        ...baseMessage,
        content: [
          {
            type: 'systemNotification',
            notificationType: 'inlineMessage',
            msg: status.message,
          },
        ],
      };
    case 'progress':
      return {
        ...baseMessage,
        content: [
          {
            type: 'systemNotification',
            notificationType: 'thinkingMessage',
            msg: status.message,
          },
        ],
      };
  }
}

function applyInteractionUpdate(
  state: AdapterState,
  sessionId: string,
  update: GooseInteractionUpdate
): AcpSessionUpdate[] {
  const { interaction } = update;
  if (interaction.type !== 'elicitation') {
    return [];
  }

  switch (interaction.state) {
    case 'pending':
      return applyPendingElicitation(state, sessionId, update);
    case 'submitted':
      return applySubmittedElicitation(state, update);
  }
}

function applyPendingElicitation(
  state: AdapterState,
  sessionId: string,
  update: GooseInteractionUpdate
): AcpSessionUpdate[] {
  const { interaction } = update;
  const replayMeta = acpReplayMessageMeta(update);
  const messageId = replayMeta.messageId ?? `acp_elicitation_${sessionId}_${interaction.id}`;
  const existing = state.messages.find(
    (message) =>
      message.id === messageId ||
      message.content.some(
        (content) =>
          content.type === 'actionRequired' &&
          content.data.actionType === 'elicitation' &&
          content.data.id === interaction.id
      )
  );

  if (!existing) {
    state.messages.push({
      id: messageId,
      role: 'assistant',
      created: replayMeta.created ?? Math.floor(Date.now() / 1000),
      content: [
        {
          type: 'actionRequired',
          data: {
            actionType: 'elicitation',
            id: interaction.id,
            message: interaction.message ?? '',
            requested_schema: interaction.requestedSchema ?? {},
          },
        },
      ],
      metadata: { ...DEFAULT_VISIBLE_MESSAGE_METADATA },
    });
  }

  return [{ type: 'messages', messages: state.messages.map(cloneMessage) }];
}

function applySubmittedElicitation(
  state: AdapterState,
  update: GooseInteractionUpdate
): AcpSessionUpdate[] {
  const { interaction } = update;
  let changed = false;

  state.messages = state.messages.flatMap((message) => {
    const content = message.content.filter((item) => {
      const shouldRemove =
        item.type === 'actionRequired' &&
        item.data.actionType === 'elicitation' &&
        item.data.id === interaction.id;
      if (shouldRemove) {
        changed = true;
      }
      return !shouldRemove;
    });
    const removedFromMessage = content.length !== message.content.length;

    if (content.length === 0 && removedFromMessage) {
      return [];
    }

    if (removedFromMessage) {
      return [{ ...message, content }];
    }

    return [message];
  });

  return changed ? [{ type: 'messages', messages: state.messages.map(cloneMessage) }] : [];
}

function applyToolCall(state: AdapterState, update: ToolCall): AcpSessionUpdate[] {
  const message = assistantMessageForReplayUpdate(state, update);
  const identity = toolIdentity(update);
  const mcpAppMeta = mcpAppMetadata(update);
  const existing = message.content.find(
    (content) => content.type === 'toolRequest' && content.id === update.toolCallId
  );

  if (!existing) {
    message.content.push({
      type: 'toolRequest',
      id: update.toolCallId,
      toolCall: {
        status: 'success',
        value: {
          name: identity.toolName ?? update.title,
          arguments: rawInputToArguments(update.rawInput),
        },
      },
      metadata: toolMetadata(update, identity),
      ...(update._meta ? { _meta: update._meta } : {}),
      ...(mcpAppMeta ? { _meta: mcpAppMeta } : {}),
    });
  }

  return [{ type: 'messages', messages: state.messages.map(cloneMessage) }];
}

function applyToolCallUpdate(state: AdapterState, update: ToolCallUpdate): AcpSessionUpdate[] {
  const identity = toolIdentity(update);
  const mcpAppMeta = mcpAppMetadata(update);
  const message = messageWithToolRequest(state, update.toolCallId);
  const request = message?.content.find(
    (content) => content.type === 'toolRequest' && content.id === update.toolCallId
  );

  if (request?.type === 'toolRequest') {
    request.metadata = {
      ...request.metadata,
      ...toolMetadata(update, identity),
    };
  }

  if (!message && update.status !== 'completed' && update.status !== 'failed') {
    assistantMessageForReplayUpdate(state, update);
  }

  if (update.status === 'completed' || update.status === 'failed') {
    const responseMessage = toolResponseMessageForReplayUpdate(state, update);
    const existingResponse = responseMessage.content.find(
      (content) => content.type === 'toolResponse' && content.id === update.toolCallId
    );

    if (!existingResponse) {
      responseMessage.content.push({
        type: 'toolResponse',
        id: update.toolCallId,
        toolResult:
          update.status === 'failed'
            ? { status: 'error', error: toolError(update) }
            : { status: 'success', value: toolResultValue(update, mcpAppMeta) },
        metadata: toolMetadata(update, identity),
      });
    }
  }

  return [{ type: 'messages', messages: state.messages.map(cloneMessage) }];
}

function applyPermissionRequest(
  state: AdapterState,
  request: RequestPermissionRequest
): AcpSessionUpdate[] {
  const toolCallId = request.toolCall.toolCallId;
  const alreadyExists = state.messages.some((message) =>
    message.content.some(
      (content) =>
        content.type === 'actionRequired' &&
        content.data.actionType === 'toolConfirmation' &&
        content.data.id === toolCallId
    )
  );

  if (!alreadyExists) {
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
  }

  return [{ type: 'messages', messages: state.messages.map(cloneMessage) }];
}

function applyContentChunk(
  state: AdapterState,
  role: Message['role'],
  update: Extract<
    SessionNotification['update'],
    { sessionUpdate: 'user_message_chunk' | 'agent_message_chunk' }
  >
): AcpSessionUpdate[] {
  const content = messageContentFromAcpContentBlock(update.content);
  if (!content) {
    return [];
  }

  const replayMeta = acpReplayMessageMeta(update);
  const id = update.messageId ?? replayMeta.messageId;
  const existing = id
    ? state.messages.find((message) => message.id === id && message.role === role)
    : lastMergeableMessageWithRole(state, role);

  if (existing) {
    const lastContent = existing.content[existing.content.length - 1];
    if (lastContent?.type === 'text' && content.type === 'text') {
      lastContent.text = mergeTextChunk(lastContent.text, content.text);
    } else if (content.type === 'image' && hasImageContent(existing, content)) {
      return [{ type: 'messages', messages: [...state.messages] }];
    } else {
      existing.content.push(content);
    }
  } else {
    state.messages.push({
      ...(id ? { id } : {}),
      role,
      created: replayMeta.created ?? Math.floor(Date.now() / 1000),
      content: [content],
      metadata: { ...DEFAULT_VISIBLE_MESSAGE_METADATA },
    });
  }

  return [{ type: 'messages', messages: [...state.messages] }];
}

function hasImageContent(message: Message, image: Extract<MessageContent, { type: 'image' }>) {
  return message.content.some(
    (content) =>
      content.type === 'image' && content.data === image.data && content.mimeType === image.mimeType
  );
}

function mergeTextChunk(existing: string, incoming: string): string {
  if (!incoming || incoming === existing || existing.endsWith(incoming)) {
    return existing;
  }

  if (!existing || incoming.startsWith(existing)) {
    return incoming;
  }

  return existing + incoming;
}

function assistantMessageForReplayUpdate(
  state: AdapterState,
  update: AcpReplayMetaContainer
): Message {
  const replayMeta = acpReplayMessageMeta(update);
  const existing = replayMeta.messageId
    ? state.messages.find(
        (message) => message.id === replayMeta.messageId && message.role === 'assistant'
      )
    : undefined;

  if (existing) {
    return existing;
  }

  const message: Message = {
    ...(replayMeta.messageId ? { id: replayMeta.messageId } : {}),
    role: 'assistant',
    created: replayMeta.created ?? Math.floor(Date.now() / 1000),
    content: [],
    metadata: { ...DEFAULT_VISIBLE_MESSAGE_METADATA },
  };
  state.messages.push(message);
  return message;
}

function messageWithToolRequest(state: AdapterState, toolCallId: string): Message | undefined {
  return state.messages.find((message) =>
    message.content.some((content) => content.type === 'toolRequest' && content.id === toolCallId)
  );
}

function toolResponseMessageForReplayUpdate(
  state: AdapterState,
  update: AcpReplayMetaContainer & { toolCallId: string }
): Message {
  const replayMeta = acpReplayMessageMeta(update);
  const existing = replayMeta.messageId
    ? state.messages.find(
        (message) => message.id === replayMeta.messageId && message.role === 'user'
      )
    : undefined;

  if (existing) {
    return existing;
  }

  const message: Message = {
    ...(replayMeta.messageId ? { id: replayMeta.messageId } : {}),
    role: 'user',
    created: replayMeta.created ?? Math.floor(Date.now() / 1000),
    content: [],
    metadata: { ...DEFAULT_VISIBLE_MESSAGE_METADATA },
  };
  state.messages.push(message);
  return message;
}

function rawInputToArguments(rawInput: unknown): Record<string, unknown> {
  return isRecord(rawInput) ? rawInput : {};
}

interface ToolIdentity {
  toolName?: string;
  extensionName?: string;
}

function toolIdentity(update: ToolCall | ToolCallUpdate): ToolIdentity {
  const goose = update._meta?.goose;
  if (!isRecord(goose) || !isRecord(goose.toolCall)) {
    return {};
  }

  return {
    toolName: typeof goose.toolCall.toolName === 'string' ? goose.toolCall.toolName : undefined,
    extensionName:
      typeof goose.toolCall.extensionName === 'string' ? goose.toolCall.extensionName : undefined,
  };
}

function toolMetadata(
  update: ToolCall | ToolCallUpdate,
  identity: ToolIdentity
): Record<string, unknown> | undefined {
  const metadata: Record<string, unknown> = {};

  if (update.title) {
    metadata.title = update.title;
  }
  if (update.status) {
    metadata.status = update.status;
  }
  if (identity.extensionName) {
    metadata.extensionName = identity.extensionName;
  }
  if (update.kind) {
    metadata.kind = update.kind;
  }
  if (update.locations) {
    metadata.locations = update.locations;
  }
  if (update.rawOutput !== undefined) {
    metadata.rawOutput = update.rawOutput;
  }
  if (update.content) {
    metadata.content = update.content;
  }

  return Object.keys(metadata).length > 0 ? metadata : undefined;
}

interface DesktopMcpAppMeta extends Record<string, unknown> {
  ui: {
    resourceUri: string;
  };
  extensionName?: string;
  toolName?: string;
}

function mcpAppMetadata(update: ToolCall | ToolCallUpdate): DesktopMcpAppMeta | undefined {
  const goose = update._meta?.goose;
  if (!isRecord(goose) || !isRecord(goose.mcpApp)) {
    return undefined;
  }

  const resourceUri = goose.mcpApp.resourceUri;
  if (typeof resourceUri !== 'string') {
    return undefined;
  }

  return {
    ui: {
      resourceUri,
    },
    extensionName:
      typeof goose.mcpApp.extensionName === 'string' ? goose.mcpApp.extensionName : undefined,
    toolName: typeof goose.mcpApp.toolName === 'string' ? goose.mcpApp.toolName : undefined,
  };
}

function toolResultValue(
  update: ToolCallUpdate,
  mcpAppMeta: DesktopMcpAppMeta | undefined
): { content: ContentBlock[]; _meta?: DesktopMcpAppMeta } {
  return {
    content: toolResultContent(update),
    ...(mcpAppMeta ? { _meta: mcpAppMeta } : {}),
  };
}

function toolResultContent(update: ToolCallUpdate): ContentBlock[] {
  const contentBlocks = update.content
    ?.filter((content) => content.type === 'content')
    .map((content) => content.content);

  if (contentBlocks?.length) {
    return contentBlocks;
  }

  if (typeof update.rawOutput === 'string') {
    return [{ type: 'text', text: update.rawOutput }];
  }

  return [];
}

function permissionPrompt(request: RequestPermissionRequest): string | undefined {
  for (const content of request.toolCall.content ?? []) {
    if (content.type === 'content' && content.content.type === 'text') {
      return content.content.text;
    }
  }

  return undefined;
}

function toolError(update: ToolCallUpdate): string {
  if (typeof update.rawOutput === 'string') {
    return update.rawOutput;
  }

  return update.title ?? 'Tool call failed';
}

function applyThoughtChunk(
  state: AdapterState,
  update: Extract<SessionNotification['update'], { sessionUpdate: 'agent_thought_chunk' }>
): AcpSessionUpdate[] {
  if (update.content.type !== 'text') {
    return [];
  }

  const replayMeta = acpReplayMessageMeta(update);
  const id = update.messageId ?? replayMeta.messageId;
  const existing = id
    ? state.messages.find((message) => message.id === id && message.role === 'assistant')
    : lastMergeableMessageWithRole(state, 'assistant');

  if (existing) {
    const lastContent = existing.content[existing.content.length - 1];
    if (lastContent?.type === 'thinking') {
      lastContent.thinking += update.content.text;
    } else {
      existing.content.push({ type: 'thinking', thinking: update.content.text, signature: '' });
    }
  } else {
    state.messages.push({
      ...(id ? { id } : {}),
      role: 'assistant',
      created: replayMeta.created ?? Math.floor(Date.now() / 1000),
      content: [{ type: 'thinking', thinking: update.content.text, signature: '' }],
      metadata: { ...DEFAULT_VISIBLE_MESSAGE_METADATA },
    });
  }

  return [{ type: 'messages', messages: [...state.messages] }];
}

function messageContentFromAcpContentBlock(content: ContentBlock): MessageContent | undefined {
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

function acpReplayMessageMeta(update: AcpReplayMetaContainer): AcpReplayMessageMeta {
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

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
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

function cloneMessage(message: Message): Message {
  return {
    ...message,
    content: message.content.map((content) => ({ ...content })),
    metadata: { ...message.metadata },
  };
}
