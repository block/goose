import type { ToolCallUpdate } from '@agentclientprotocol/sdk';
import type { NotificationEvent } from '../../types/message';
import type { AcpChatStateChange } from './shared';
import { isRecord } from './shared';

type ToolNotification =
  | {
      type: 'message';
      params: LoggingMessageNotificationParams;
    }
  | {
      type: 'progress';
      params: ProgressNotificationParams;
    };

type LoggingMessageNotificationParams = {
  level: string;
  logger?: string;
  data: unknown;
};

type ProgressNotificationParams = {
  progressToken: string | number;
  progress: number;
  total?: number;
  message?: string;
};

export function toolNotificationChange(
  update: ToolCallUpdate
): Extract<AcpChatStateChange, { type: 'notification' }> | undefined {
  const toolNotification = parseToolNotification(update._meta);
  if (!toolNotification) {
    return undefined;
  }

  return {
    type: 'notification',
    notification: toNotificationEvent(update.toolCallId, toolNotification),
  };
}

function parseToolNotification(meta: unknown): ToolNotification | undefined {
  if (!isRecord(meta)) {
    return undefined;
  }

  const toolNotification = meta.toolNotification;
  if (!isRecord(toolNotification)) {
    return undefined;
  }

  if (toolNotification.type === 'message') {
    const params = parseLoggingMessageParams(toolNotification.params);
    return params ? { type: 'message', params } : undefined;
  }

  if (toolNotification.type === 'progress') {
    const params = parseProgressParams(toolNotification.params);
    return params ? { type: 'progress', params } : undefined;
  }

  return undefined;
}

function parseLoggingMessageParams(value: unknown): LoggingMessageNotificationParams | undefined {
  if (!isRecord(value) || typeof value.level !== 'string' || !('data' in value)) {
    return undefined;
  }

  return {
    level: value.level,
    ...(typeof value.logger === 'string' ? { logger: value.logger } : {}),
    data: value.data,
  };
}

function parseProgressParams(value: unknown): ProgressNotificationParams | undefined {
  if (
    !isRecord(value) ||
    (typeof value.progressToken !== 'string' && typeof value.progressToken !== 'number') ||
    typeof value.progress !== 'number'
  ) {
    return undefined;
  }

  return {
    progressToken: value.progressToken,
    progress: value.progress,
    ...(typeof value.total === 'number' ? { total: value.total } : {}),
    ...(typeof value.message === 'string' ? { message: value.message } : {}),
  };
}

function toNotificationEvent(
  toolCallId: string,
  toolNotification: ToolNotification
): NotificationEvent {
  return {
    type: 'Notification',
    request_id: toolCallId,
    message: {
      method:
        toolNotification.type === 'message'
          ? 'notifications/message'
          : 'notifications/progress',
      params: toolNotification.params,
    },
  };
}
