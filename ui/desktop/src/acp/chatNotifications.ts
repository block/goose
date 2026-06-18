import type { GooseSessionNotification_unstable } from '@aaif/goose-sdk';
import type { SessionNotification } from '@agentclientprotocol/sdk';
import { USE_ACP_CHAT } from '../acpChatFeatureFlag';
import { AppEvents } from '../constants/events';
import { acpChatSessionStore } from './chatSessionStore';

export function handleAcpSessionNotification(notification: SessionNotification): Promise<void> {
  if (USE_ACP_CHAT) {
    const previousName = acpChatSessionStore.getSnapshot(notification.sessionId)?.session?.name;
    const snapshot = acpChatSessionStore.applyAcpSessionNotification(notification);
    const newName = snapshot.session?.name;

    if (newName && newName !== previousName) {
      window.dispatchEvent(
        new CustomEvent(AppEvents.SESSION_RENAMED, {
          detail: { sessionId: notification.sessionId, newName },
        })
      );
    }
  }
  return Promise.resolve();
}

export function handleAcpGooseSessionNotification(
  notification: GooseSessionNotification_unstable
): Promise<void> {
  if (USE_ACP_CHAT) {
    acpChatSessionStore.applyAcpGooseSessionNotification(notification);
  }
  return Promise.resolve();
}
