import type { GooseSessionNotification_unstable } from '@aaif/goose-sdk';
import type { SessionNotification } from '@agentclientprotocol/sdk';
import { USE_ACP_CHAT } from '../acpChatFeatureFlag';
import { acpChatSessionStore } from './chatSessionStore';

export function handleAcpSessionNotification(notification: SessionNotification): Promise<void> {
  if (USE_ACP_CHAT) {
    acpChatSessionStore.applyAcpSessionNotification(notification);
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
