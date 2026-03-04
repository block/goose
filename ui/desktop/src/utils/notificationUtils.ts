import type { NotificationEvent } from '@/types/message';

export type NotificationMethod = string;

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object') return null;
  return value as Record<string, unknown>;
}

export function getNotificationMethod(message: unknown): NotificationMethod | null {
  const m = asRecord(message);
  if (!m) return null;

  const directMethod = m.method;
  if (typeof directMethod === 'string') return directMethod;

  const custom = asRecord(m.custom);
  const customMethod = custom?.method;
  if (typeof customMethod === 'string') return customMethod;

  const customNotification = asRecord(m.CustomNotification);
  const customNotificationMethod = customNotification?.method;
  if (typeof customNotificationMethod === 'string') return customNotificationMethod;

  // Some serializers use snake_case or variant wrapping.
  const customNotificationSnake = asRecord(m.custom_notification);
  const customNotificationSnakeMethod = customNotificationSnake?.method;
  if (typeof customNotificationSnakeMethod === 'string') return customNotificationSnakeMethod;

  return null;
}

export function getNotificationParams(message: unknown): unknown {
  const m = asRecord(message);
  if (!m) return undefined;

  if ('params' in m) return m.params;

  const custom = asRecord(m.custom);
  if (custom && 'params' in custom) return custom.params;

  const customNotification = asRecord(m.CustomNotification);
  if (customNotification && 'params' in customNotification) return customNotification.params;

  const customNotificationSnake = asRecord(m.custom_notification);
  if (customNotificationSnake && 'params' in customNotificationSnake) {
    return customNotificationSnake.params;
  }

  return undefined;
}

export function isGooseActivityEvent(event: NotificationEvent): boolean {
  return getNotificationMethod(event.message) === 'goose/activity';
}

export function getGooseActivityFields(message: unknown): { phase: string; text: string } | null {
  const params = getNotificationParams(message);
  const p = asRecord(params) ?? {};
  const phase = typeof p.phase === 'string' ? p.phase : 'activity';
  const text = typeof p.text === 'string' ? p.text.trim() : '';

  if (!text) return null;

  return { phase, text };
}
