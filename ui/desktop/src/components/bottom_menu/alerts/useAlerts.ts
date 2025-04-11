import { useState, useCallback } from 'react';
import { Alert, AlertType } from './types';

export const useAlerts = () => {
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
