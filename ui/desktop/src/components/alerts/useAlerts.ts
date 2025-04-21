import { useState, useCallback } from 'react';
import { Alert, AlertType } from './types';

interface UseAlerts {
  alerts: Alert[];
  addAlert: (
    type: AlertType,
    message: string,
    action?: { text: string; onClick: () => void }
  ) => void;
  removeAlert: (index: number) => void;
  clearAlerts: () => void;
}

export const useAlerts = (): UseAlerts => {
  const [alerts, setAlerts] = useState<Alert[]>([]);

  const addAlert = useCallback(
    (type: AlertType, message: string, action?: { text: string; onClick: () => void }) => {
      setAlerts((prev) => [...prev, { type, message, action }]);
    },
    []
  );

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
