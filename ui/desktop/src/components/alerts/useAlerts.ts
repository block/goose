import { useState, useCallback } from 'react';
import { Alert } from './types';

interface UseAlerts {
  alerts: Alert[];
  addAlert: (options: Alert) => void;
  removeAlert: (index: number) => void;
  clearAlerts: () => void;
}

export const useAlerts = (): UseAlerts => {
  const [alerts, setAlerts] = useState<Alert[]>([]);

  const addAlert = useCallback((options: Alert) => {
    setAlerts((prev) => [...prev, options]);
  }, []);

  const removeAlert = useCallback((index: number) => {
    setAlerts((prev) => prev.filter((_, i) => i !== index));
  }, []);

  const clearAlerts = useCallback(() => {
    setAlerts([]);
  }, []);

  return {
    alerts,
    addAlert,
    removeAlert,
    clearAlerts,
  };
};
