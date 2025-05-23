/**
 * System notification permission handling
 *
 * Provides platform-specific permission checks and requests:
 * - macOS: Uses native notification center permissions
 * - Other platforms: Uses web notification permissions
 */

import { notificationsNative } from './native';
import type { NotificationPermissions } from './types';

/**
 * Check system-level notification permissions
 *
 * @returns Object containing permission status for different notification levels
 */
export async function checkSystemPermissions(): Promise<NotificationPermissions> {
  if (process.platform !== 'darwin') {
    return { system: false, notificationCenter: false, app: false };
  }

  try {
    const isAuthorized = await notificationsNative.checkSettings();
    return {
      system: isAuthorized,
      notificationCenter: isAuthorized,
      app: isAuthorized,
    };
  } catch (error) {
    console.error('[Notifications] Permission check error:', error);
    return { system: false, notificationCenter: false, app: false };
  }
}

/**
 * Request notification permissions from the system
 * Only works on platforms that support permission requests
 *
 * @returns Promise<boolean> True if permissions were granted
 */
export async function requestNotificationPermissions(): Promise<boolean> {
  try {
    // Check web notification permissions first
    if ('Notification' in globalThis) {
      const permission = await globalThis.Notification.requestPermission();
      if (permission !== 'granted') {
        return false;
      }
    }

    // Then check native permissions
    if (process.platform === 'darwin') {
      return await notificationsNative.requestPermission();
    }

    return true;
  } catch (error) {
    console.error('[Notifications] Permission request error:', error);
    return false;
  }
}
