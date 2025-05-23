/**
 * Notification System Implementation
 *
 * Provides a queue-based notification system with:
 * - Platform-specific native notifications
 * - Sound support (platform dependent)
 * - Focus-aware display logic
 * - Configurable notification types
 */

import { NotificationOptions, NotificationSettings, NotificationType } from './types';
import { validateNotificationSettings } from './validation';

// === Notification Queue Management ===
class NotificationQueue {
  private queue: NotificationOptions[] = [];
  private isProcessing = false;
  private readonly DEFAULT_DELAY = 500;

  async add(notification: NotificationOptions) {
    if (!this.validateNotification(notification)) {
      throw new Error('Invalid notification options');
    }

    this.queue.push(notification);
    if (!this.isProcessing) {
      await this.processQueue();
    }
  }

  private validateNotification(notification: NotificationOptions): boolean {
    if (!notification || typeof notification !== 'object') return false;

    const requiredFields: (keyof NotificationOptions)[] = ['title', 'body', 'type'];
    for (const field of requiredFields) {
      if (!notification[field]) return false;
    }

    if (!['taskCompletion', 'error'].includes(notification.type)) return false;

    return true;
  }

  private async processQueue() {
    if (this.queue.length === 0) {
      this.isProcessing = false;
      return;
    }

    this.isProcessing = true;
    const notification = this.queue.shift();

    if (notification) {
      try {
        await this.showNotification(notification);
      } catch (error) {
        console.error('[Notifications] Failed to show notification:', error);
        // Add to error metrics/monitoring here if available
      } finally {
        const settings = this.getValidatedSettings();
        const delay = settings?.delayBetweenNotifications ?? this.DEFAULT_DELAY;
        setTimeout(() => this.processQueue(), delay);
      }
    }
  }

  private getValidatedSettings(): NotificationSettings | null {
    try {
      const settings = window.electron.getConfig().notifications as NotificationSettings;
      return validateNotificationSettings(settings) ? settings : null;
    } catch (error) {
      console.error('[Notifications] Settings validation error:', error);
      return null;
    }
  }

  private async checkWindowFocus(): Promise<boolean> {
    try {
      // Check all windows through IPC
      return await window.electron.isAnyWindowFocused();
    } catch (error) {
      console.error('[Notifications] Window focus check error:', error);
      return false;
    }
  }

  private async showNotification(options: NotificationOptions) {
    try {
      const settings = this.getValidatedSettings();
      if (!settings) {
        console.warn('[Notifications] Invalid settings, using defaults');
        return;
      }

      if (!(await this.shouldShowNotification(settings, options))) {
        return;
      }

      const platformSpecificOptions = await this.getPlatformSpecificOptions(options, settings);

      await window.electron.showNotification(platformSpecificOptions);
    } catch (error) {
      console.error('[Notifications] Show error:', error);
      throw error;
    }
  }

  private async shouldShowNotification(
    settings: NotificationSettings,
    options: NotificationOptions
  ): Promise<boolean> {
    if (!settings?.enabled) {
      return false;
    }

    if (!settings.types.includes(options.type)) {
      return false;
    }

    if (settings.onlyWhenUnfocused && (await this.checkWindowFocus())) {
      return false;
    }

    return true;
  }

  private async getPlatformSpecificOptions(
    options: NotificationOptions,
    settings: NotificationSettings
  ): Promise<NotificationOptions & { sound?: string; silent?: boolean }> {
    const platform = window.electron.platform;
    const sound = settings.sounds[options.type];

    return {
      ...options,
      sound: platform === 'darwin' ? sound : undefined,
      silent: platform === 'darwin' ? sound === 'default' : undefined,
    };
  }
}

// === Singleton Instance ===
export const notificationQueue = new NotificationQueue();

// === Helper Functions ===
export function showNotification(
  type: NotificationType,
  title: string,
  body: string,
  options: Partial<NotificationOptions> = {}
) {
  return notificationQueue.add({
    type,
    title,
    body,
    ...options,
  });
}

export const showTaskCompleteNotification = (taskDescription: string) =>
  showNotification('taskCompletion', 'Task Complete', `Completed: ${taskDescription}`);

export const showErrorNotification = (errorMessage: string) =>
  showNotification('error', 'Error', `Error: ${errorMessage}`);

export const showLongRunningTaskNotification = (taskDescription: string) =>
  showNotification('taskCompletion', 'Long Running Task Complete', `Completed: ${taskDescription}`);
