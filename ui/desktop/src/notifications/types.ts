/**
 * Type definitions for the notification system
 */

// Constants for notification settings
export const NOTIFICATION_CONSTANTS = {
  DEFAULT_MIN_NOTIFICATION_DELAY: 0,
  DEFAULT_MAX_NOTIFICATION_DELAY: 10000,
  DEFAULT_NOTIFICATION_DELAY: 500,
} as const;

export type NotificationType =
  | 'taskCompletion' // When Goose completes a task (including long-running tasks)
  | 'error'; // When something goes wrong or needs attention

export interface NotificationAction {
  type: string;
  text: string;
}

/**
 * Base notification options that are common across the system
 */
export interface BaseNotificationOptions {
  title: string;
  body: string;
  type: NotificationType;
  sound?: string;
  onClick?: () => void;
}

/**
 * Extended notification options used in the main process
 */
export interface NotificationData extends BaseNotificationOptions {
  icon?: string;
  silent?: boolean;
  actions?: NotificationAction[];
}

/**
 * Configuration for the notification system.
 * Stored in the user's config and managed through settings UI.
 */
export interface NotificationSettings {
  /** Whether notifications are enabled system-wide */
  enabled: boolean;
  /** List of enabled notification types */
  types: NotificationType[];
  /** Only show notifications when app is not focused */
  onlyWhenUnfocused: boolean;
  /** Sound settings per notification type */
  sounds: Record<NotificationType, string>;
  /** Delay between consecutive notifications in milliseconds */
  delayBetweenNotifications: number;
}

/**
 * System permission status for notifications
 */
export interface NotificationPermissions {
  /** Whether notifications are enabled at the system level */
  system: boolean;
  /** Whether notifications are enabled in notification center */
  notificationCenter: boolean;
  /** Whether the app is allowed to show notifications */
  app: boolean;
}
