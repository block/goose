import type { GoslingSessionNotification_unstable } from '@repo-makeover/gosling-sdk';
import type { SessionNotification } from '@agentclientprotocol/sdk';
import { AppEvents } from '../constants/events';
import { acpChatSessionActions, acpChatSessionStore } from './chatSessionStore';

export function handleAcpSessionNotification(notification: SessionNotification): Promise<void> {
  const sessionNameBeforeNotification = acpChatSessionStore.getSnapshot(
    notification.sessionId
  )?.session?.name;
  const updatedName =
    notification.update.sessionUpdate === 'session_info_update'
      ? notification.update.title
      : undefined;
  acpChatSessionActions.applyAcpSessionNotification(notification);

  if (updatedName && updatedName !== sessionNameBeforeNotification) {
    window.dispatchEvent(
      new CustomEvent(AppEvents.SESSION_RENAMED, {
        detail: { sessionId: notification.sessionId, newName: updatedName },
      })
    );
  }

  return Promise.resolve();
}

export function handleAcpGoslingSessionNotification(
  notification: GoslingSessionNotification_unstable
): Promise<void> {
  acpChatSessionActions.applyAcpGoslingSessionNotification(notification);
  return Promise.resolve();
}
