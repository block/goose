import { useConfig } from '../../ConfigContext';
import type { NotificationSettings, NotificationType } from '../../../notifications/types';

export function useNotificationSettings() {
  const { config, upsert } = useConfig();

  // Get settings from config or use defaults
  const settings: NotificationSettings = {
    enabled: false,
    types: ['taskCompletion', 'error'],
    onlyWhenUnfocused: true,
    sounds: {
      taskCompletion: 'default',
      error: 'default',
    },
    delayBetweenNotifications: 500,
    ...(config as { notifications?: Partial<NotificationSettings> }).notifications,
  };

  // Update settings in config
  const updateSettings = async (newSettings: Partial<NotificationSettings>) => {
    window.electron.logInfo(`[Notifications] Updating settings: ${JSON.stringify(newSettings)}`);
    await upsert(
      'notifications',
      {
        ...settings,
        ...newSettings,
      },
      false
    );
  };

  // Individual setting updaters
  const setEnabled = async (enabled: boolean) => {
    await updateSettings({ enabled });
  };

  const toggleType = async (type: NotificationType) => {
    const types = settings.types.includes(type)
      ? settings.types.filter((t) => t !== type)
      : [...settings.types, type];
    await updateSettings({ types });
  };

  const setOnlyWhenUnfocused = async (onlyWhenUnfocused: boolean) => {
    await updateSettings({ onlyWhenUnfocused });
  };

  const setSound = async (type: NotificationType, sound: string) => {
    window.electron.logInfo(`[Notifications] Setting ${type} sound to: ${sound}`);
    const newSounds = {
      ...settings.sounds,
      [type]: sound,
    };
    await updateSettings({ sounds: newSounds });
  };

  const setDelayBetweenNotifications = async (delay: number) => {
    await updateSettings({ delayBetweenNotifications: delay });
  };

  return {
    // Current settings
    isEnabled: settings.enabled,
    types: settings.types,
    onlyWhenUnfocused: settings.onlyWhenUnfocused,
    sounds: settings.sounds,
    delayBetweenNotifications: settings.delayBetweenNotifications,

    // Updaters
    setEnabled,
    toggleType,
    setOnlyWhenUnfocused,
    setSound,
    setDelayBetweenNotifications,
  };
}
