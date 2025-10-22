import { useState, useEffect, useCallback } from 'react';

interface LogSizeInfo {
  total_bytes: number;
  total_mb: number;
  total_gb: number;
  file_count: number;
  log_path: string;
}

interface LogMonitorSettings {
  enabled: boolean;
  checkInterval: number; // in milliseconds
  warningThresholdGB: number;
  criticalThresholdGB: number;
  lastCheckTime: number;
}

interface LogMonitorResult {
  logSize: LogSizeInfo | null;
  isChecking: boolean;
  warningLevel: 'none' | 'warning' | 'critical';
  settings: LogMonitorSettings;
  updateSettings: (settings: Partial<LogMonitorSettings>) => void;
  checkNow: () => Promise<void>;
  error: string | null;
}

const DEFAULT_SETTINGS: LogMonitorSettings = {
  enabled: true,
  checkInterval: 60000, // 60 seconds
  warningThresholdGB: 1,
  criticalThresholdGB: 5,
  lastCheckTime: 0,
};

const SETTINGS_KEY = 'log_monitor_settings';

function loadSettings(): LogMonitorSettings {
  try {
    const stored = localStorage.getItem(SETTINGS_KEY);
    if (stored) {
      return { ...DEFAULT_SETTINGS, ...JSON.parse(stored) };
    }
  } catch (error) {
    console.error('Failed to load log monitor settings:', error);
  }
  return DEFAULT_SETTINGS;
}

function saveSettings(settings: LogMonitorSettings): void {
  try {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
  } catch (error) {
    console.error('Failed to save log monitor settings:', error);
  }
}

export function useLogMonitor(): LogMonitorResult {
  const [logSize, setLogSize] = useState<LogSizeInfo | null>(null);
  const [isChecking, setIsChecking] = useState(false);
  const [settings, setSettings] = useState<LogMonitorSettings>(loadSettings);
  const [error, setError] = useState<string | null>(null);

  const checkLogSize = useCallback(async () => {
    if (!settings.enabled) return;

    setIsChecking(true);
    setError(null);

    try {
      const size = await window.electron.getLogSize();
      setLogSize(size);

      // Update last check time
      const newSettings = { ...settings, lastCheckTime: Date.now() };
      setSettings(newSettings);
      saveSettings(newSettings);
    } catch (err) {
      setError('Failed to check log size');
      console.error('Error checking log size:', err);
    } finally {
      setIsChecking(false);
    }
  }, [settings]);

  const updateSettings = useCallback((newSettings: Partial<LogMonitorSettings>) => {
    setSettings((prev) => {
      const updated = { ...prev, ...newSettings };
      saveSettings(updated);
      return updated;
    });
  }, []);

  const checkNow = useCallback(async () => {
    await checkLogSize();
  }, [checkLogSize]);

  // Initial check on mount
  useEffect(() => {
    checkLogSize();
  }, [checkLogSize]);

  // Periodic checking
  useEffect(() => {
    if (!settings.enabled) return;

    const interval = setInterval(() => {
      checkLogSize();
    }, settings.checkInterval);

    return () => clearInterval(interval);
  }, [settings.enabled, settings.checkInterval, checkLogSize]);

  const warningLevel: 'none' | 'warning' | 'critical' = logSize
    ? logSize.total_gb >= settings.criticalThresholdGB
      ? 'critical'
      : logSize.total_gb >= settings.warningThresholdGB
        ? 'warning'
        : 'none'
    : 'none';

  return {
    logSize,
    isChecking,
    warningLevel,
    settings,
    updateSettings,
    checkNow,
    error,
  };
}
