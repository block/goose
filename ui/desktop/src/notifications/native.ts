/**
 * Native notification module integration
 *
 * Provides platform-specific notification functionality:
 * - macOS: Uses native notification center
 * - Other platforms: Falls back to web notifications
 */

import { join } from 'path';
import { app } from 'electron';
import * as fs from 'fs';

// === Module Resolution ===
function resolveNativeModulePath(): string | null {
  const candidates = [
    join(__dirname, '..', 'native/notifications/build/Release/notifications.node'),
    join(process.resourcesPath, 'native/notifications/build/Release/notifications.node'),
    join(app.getAppPath(), '../native/notifications/build/Release/notifications.node'),
  ];

  for (const candidatePath of candidates) {
    if (fs.existsSync(candidatePath)) {
      console.log('[Notifications] Found native module at:', candidatePath);
      return candidatePath;
    }
  }

  console.error('[Notifications] Native module not found');
  return null;
}

// === Native Interface ===
export interface NotificationsNative {
  checkSettings(): Promise<boolean>;
  requestPermission(): Promise<boolean>;
}

// === Module Loading ===
const notifications =
  process.platform === 'darwin'
    ? (() => {
        const modulePath = resolveNativeModulePath();
        return modulePath ? require(modulePath) : createFallbackModule();
      })()
    : createFallbackModule();

function createFallbackModule(): NotificationsNative {
  return {
    checkSettings: () => Promise.resolve(false),
    requestPermission: () => Promise.resolve(false),
  };
}

// === Typed Wrapper ===
export const notificationsNative: NotificationsNative = {
  checkSettings: async () => {
    try {
      console.log('[Notifications] Checking settings');
      return await notifications.checkSettings();
    } catch (error) {
      console.error('[Notifications] Check settings error:', error);
      return false;
    }
  },

  requestPermission: async () => {
    try {
      console.log('[Notifications] Requesting permission');
      return await notifications.requestPermission();
    } catch (error) {
      console.error('[Notifications] Request permission error:', error);
      return false;
    }
  },
};
