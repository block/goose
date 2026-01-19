import { listApps, GooseApp } from '../api';

/**
 * Platform Events Module
 *
 * Handles platform event notifications from the backend via MCP streaming.
 * Backend sends Notification events which get converted to window CustomEvents,
 * then routed to extension-specific handlers.
 */

// Type definitions for platform events
interface PlatformEventData {
  extension: string;
  sessionId?: string;
  [key: string]: unknown;
}

interface AppsEventData extends PlatformEventData {
  app_name?: string;
  sessionId: string;
}

type PlatformEventHandler = (eventType: string, data: PlatformEventData) => Promise<void>;

// Extension-specific event handlers

async function handleAppsEvent(eventType: string, eventData: PlatformEventData): Promise<void> {
  const { app_name, sessionId } = eventData as AppsEventData;

  console.log(`[platform_events] Handling apps event: ${eventType}, app_name: '${app_name}'`);

  if (!sessionId) {
    console.warn('No sessionId in apps platform event, skipping');
    return;
  }

  // Fetch fresh apps list to get latest state
  const response = await listApps({
    throwOnError: false,
    query: { session_id: sessionId },
  });

  const apps = response.data?.apps || [];
  console.log(
    `[platform_events] Fetched ${apps.length} apps:`,
    apps.map((a: GooseApp) => a.name)
  );

  const targetApp = apps.find((app: GooseApp) => app.name === app_name);
  console.log(`[platform_events] Target app found:`, targetApp ? 'YES' : 'NO');

  switch (eventType) {
    case 'app_created':
      // Open the newly created app
      if (targetApp) {
        await window.electron.launchApp(targetApp).catch((err) => {
          console.error('Failed to launch newly created app:', err);
        });
      }
      break;

    case 'app_updated':
      // Refresh the app if it's currently open
      if (targetApp) {
        await window.electron.refreshApp(targetApp).catch((err) => {
          console.error('Failed to refresh updated app:', err);
        });
      }
      break;

    case 'app_deleted':
      // Close the app if it's currently open
      if (app_name) {
        await window.electron.closeApp(app_name).catch((err) => {
          console.error('Failed to close deleted app:', err);
        });
      }
      break;

    default:
      console.warn(`Unknown apps event type: ${eventType}`);
  }
}

// Registry mapping extension name to handler function
const EXTENSION_HANDLERS: Record<string, PlatformEventHandler> = {
  apps: handleAppsEvent,
  // Future extensions can register handlers here
};

/**
 * Check if a notification is a platform event and dispatch it as a window CustomEvent.
 * Called from useChatStream when receiving Notification MessageEvents.
 */
export function maybe_handle_platform_event(notification: unknown, sessionId: string): void {
  if (notification && typeof notification === 'object' && 'method' in notification) {
    const msg = notification as { method?: string; params?: unknown };
    if (msg.method === 'platform_event' && msg.params) {
      // Dispatch window event with sessionId included
      window.dispatchEvent(
        new CustomEvent('platform-event', {
          detail: { ...msg.params, sessionId },
        })
      );
    }
  }
}

/**
 * Register global platform event handlers.
 * Call this from AppInner to set up listeners that are always active.
 * Returns cleanup function to remove listeners.
 */
export function registerPlatformEventHandlers(): () => void {
  const handler = (event: Event) => {
    const customEvent = event as CustomEvent;
    const { extension, event_type, ...data } = customEvent.detail;

    const extensionHandler = EXTENSION_HANDLERS[extension];
    if (extensionHandler) {
      extensionHandler(event_type, { ...data, extension }).catch((err) => {
        console.error(`Platform event handler failed for ${extension}:`, err);
      });
    }
  };

  window.addEventListener('platform-event', handler);
  return () => window.removeEventListener('platform-event', handler);
}
