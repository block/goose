import type { GooseSessionNotification_unstable } from '@aaif/goose-sdk';
import type { SessionNotification } from '@agentclientprotocol/sdk';
import { AppEvents } from '../constants/events';
import { maybeHandlePlatformEvent } from '../utils/platform_events';
import type { ExtensionLoadResult } from '../types/extensions';
import { showExtensionLoadResults } from '../utils/extensionErrorUtils';
import { toolNotificationEvent } from './adapter/toolNotifications';
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
  maybeHandleLivePlatformEvent(notification);

  if (updatedName && updatedName !== sessionNameBeforeNotification) {
    window.dispatchEvent(
      new CustomEvent(AppEvents.SESSION_RENAMED, {
        detail: { sessionId: notification.sessionId, newName: updatedName },
      })
    );
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
  acpChatSessionActions.applyAcpGooseSessionNotification(notification);
  if (notification.update.sessionUpdate === 'extensions_loaded') {
    handleExtensionsLoaded(notification.sessionId, notification.update.extensionResults);
  }
  return Promise.resolve();
}

function handleExtensionsLoaded(sessionId: string, extensionResults: ExtensionLoadResult[]): void {
  showExtensionLoadResults(extensionResults);
  window.dispatchEvent(
    new CustomEvent(AppEvents.SESSION_EXTENSIONS_LOADED, {
      detail: { sessionId, extensionResults },
    })
  );
}
