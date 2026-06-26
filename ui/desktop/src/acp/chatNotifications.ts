import type { GooseSessionNotification_unstable } from '@aaif/goose-sdk';
import type { SessionNotification } from '@agentclientprotocol/sdk';
import { USE_ACP_CHAT } from '../acpChatFeatureFlag';
import { AppEvents } from '../constants/events';
import { maybeHandlePlatformEvent } from '../utils/platform_events';
import { toolNotificationEvent } from './adapter/toolNotifications';
import { acpChatSessionActions, acpChatSessionStore } from './chatSessionStore';

export function handleAcpSessionNotification(notification: SessionNotification): Promise<void> {
  if (USE_ACP_CHAT) {
    const sessionNameBeforeNotification = acpChatSessionStore.getSnapshot(
      notification.sessionId
    )?.session?.name;
    const updatedName =
      notification.update.sessionUpdate === 'session_info_update'
        ? notification.update.title
        : undefined;
    acpChatSessionActions.applyAcpSessionNotification(notification);
    maybeHandleLivePlatformEvent(notification);

    if (updatedName && updatedName !== sessionNameBeforeNotification) {
      window.dispatchEvent(
        new CustomEvent(AppEvents.SESSION_RENAMED, {
          detail: { sessionId: notification.sessionId, newName: updatedName },
        })
      );
    }
  }
  return Promise.resolve();
}

function maybeHandleLivePlatformEvent(notification: SessionNotification): void {
  const update = notification.update;
  if (
    update.sessionUpdate !== 'tool_call_update' ||
    update.status === 'completed' ||
    update.status === 'failed'
  ) {
    return;
  }

  const event = toolNotificationEvent(update);
  if (event?.message.method === 'platform_event') {
    maybeHandlePlatformEvent(event.message, notification.sessionId);
  }
}

export function handleAcpGooseSessionNotification(
  notification: GooseSessionNotification_unstable
): Promise<void> {
  if (USE_ACP_CHAT) {
    acpChatSessionActions.applyAcpGooseSessionNotification(notification);
  }
  return Promise.resolve();
}
