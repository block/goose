/**
 * Validation utilities for notification system
 */

import { NotificationSettings, NotificationType, NOTIFICATION_CONSTANTS } from './types';

const VALID_NOTIFICATION_TYPES: NotificationType[] = ['taskCompletion', 'error'];

export function validateNotificationSettings(settings: unknown): settings is NotificationSettings {
  if (!settings || typeof settings !== 'object') {
    return false;
  }

  const s = settings as NotificationSettings;

  // Check required boolean fields
  if (typeof s.enabled !== 'boolean' || typeof s.onlyWhenUnfocused !== 'boolean') {
    return false;
  }

  // Validate notification types array
  if (
    !Array.isArray(s.types) ||
    !s.types.every((type) => VALID_NOTIFICATION_TYPES.includes(type))
  ) {
    return false;
  }

  // Validate sounds configuration
  if (!s.sounds || typeof s.sounds !== 'object') {
    return false;
  }

  for (const type of VALID_NOTIFICATION_TYPES) {
    if (typeof s.sounds[type] !== 'string') {
      return false;
    }
  }

  // Validate delay
  if (
    typeof s.delayBetweenNotifications !== 'number' ||
    s.delayBetweenNotifications < NOTIFICATION_CONSTANTS.DEFAULT_MIN_NOTIFICATION_DELAY ||
    s.delayBetweenNotifications > NOTIFICATION_CONSTANTS.DEFAULT_MAX_NOTIFICATION_DELAY
  ) {
    return false;
  }

  return true;
}

export function createDefaultSettings(): NotificationSettings {
  return {
    enabled: true,
    types: ['taskCompletion', 'error'],
    onlyWhenUnfocused: true,
    sounds: {
      taskCompletion: 'default',
      error: 'default',
    },
    delayBetweenNotifications: NOTIFICATION_CONSTANTS.DEFAULT_NOTIFICATION_DELAY,
  };
}
