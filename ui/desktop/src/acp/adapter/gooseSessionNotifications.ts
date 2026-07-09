import type { GooseSessionNotification_unstable } from '@aaif/goose-sdk';
import type { TokenState } from '../../types/chat';
import type { MessageUsage } from '../../types/message';
import { type AcpChatStateChange, type AdapterState, messagesChange } from './shared';

export function applyGooseSessionNotification(
  state: AdapterState,
  notification: GooseSessionNotification_unstable
): AcpChatStateChange[] {
  const update = notification.update;

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
            ...(update.accumulatedCost !== undefined
              ? { accumulatedCost: update.accumulatedCost }
              : {}),
          },
        },
      ];
    case 'status_message':
      return applyStatusMessage(state, notification.sessionId, update);
    case 'message_usage':
      return applyMessageUsage(state, update);
    default:
      return [];
  }
}

function applyStatusMessage(
  state: AdapterState,
  sessionId: string,
  update: Extract<GooseSessionNotification_unstable['update'], { sessionUpdate: 'status_message' }>
): AcpChatStateChange[] {
  if (update.status.type === 'progress') {
    return [{ type: 'progressMessage', message: update.status.message }];
  }

  state.messages.push({
    id: `acp_status_${sessionId}_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`,
    role: 'assistant',
    created: Math.floor(Date.now() / 1000),
    content: [
      {
        type: 'systemNotification',
        notificationType: 'inlineMessage',
        msg: update.status.message,
      },
    ],
    metadata: {
      userVisible: true,
      agentVisible: false,
    },
  });

  return messagesChange(state);
}

function applyMessageUsage(
  state: AdapterState,
  update: Extract<GooseSessionNotification_unstable['update'], { sessionUpdate: 'message_usage' }>
): AcpChatStateChange[] {
  const tokenState: Partial<TokenState> = {};
  const { inputTokens, outputTokens, totalTokens, cacheReadTokens, cacheWriteTokens } =
    update.usage;
  if (inputTokens != null) tokenState.inputTokens = inputTokens;
  if (outputTokens != null) tokenState.outputTokens = outputTokens;
  if (totalTokens != null) tokenState.totalTokens = totalTokens;
  if (cacheReadTokens != null) tokenState.cacheReadTokens = cacheReadTokens;
  if (cacheWriteTokens != null) tokenState.cacheWriteTokens = cacheWriteTokens;

  const tokenStateChange: AcpChatStateChange | null =
    update.usage.isCompaction || Object.keys(tokenState).length === 0
      ? null
      : { type: 'tokenState', tokenState };

  // Live tool-call turns carry a server-side id the client never saw, so an
  // id miss falls back to the most recent assistant message with provider
  // content - skipping client-made rows (approval cards, status notices).
  const byId = update.messageId
    ? state.messages.find((message) => message.id === update.messageId)
    : undefined;
  const target =
    byId ??
    [...state.messages]
      .reverse()
      .find(
        (message) =>
          message.role === 'assistant' &&
          message.content.some(
            (content) => content.type !== 'actionRequired' && content.type !== 'systemNotification'
          )
      );

  if (!target) {
    return tokenStateChange ? [tokenStateChange] : [];
  }

  const usage: MessageUsage = {
    inputTokens: update.usage.inputTokens,
    outputTokens: update.usage.outputTokens,
    totalTokens: update.usage.totalTokens,
    cacheReadTokens: update.usage.cacheReadTokens,
    cacheWriteTokens: update.usage.cacheWriteTokens,
    cost: update.usage.cost,
    costSource: update.usage.costSource,
    elapsedMs: update.usage.elapsedMs,
    timeToFirstTokenMs: update.usage.timeToFirstTokenMs,
    isCompaction: update.usage.isCompaction,
  };

  target.metadata = { ...target.metadata, usage };
  return tokenStateChange ? [...messagesChange(state), tokenStateChange] : messagesChange(state);
}
