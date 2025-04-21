import { useState, useCallback } from 'react';
import { Alert, AlertType } from './types';

interface AlertOptions {
  type: AlertType;
  message: string;
  action?: {
    text: string;
    onClick: () => void;
  };
  autoShow?: boolean;
}

interface UseAlerts {
  alerts: Alert[];
  addAlert: (options: AlertOptions) => void;
  removeAlert: (index: number) => void;
  clearAlerts: () => void;
}

export const useAlerts = (): UseAlerts => {
  const [alerts, setAlerts] = useState<Alert[]>([]);

  const addAlert = useCallback((options: AlertOptions) => {
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
