import { useState, useCallback } from 'react';
import { Config } from '../api/config';
import { toast } from 'react-toastify';

export interface UseConfigOptions {
  onError?: (error: Error) => void;
  showToasts?: boolean;
}

export function useConfig(options: UseConfigOptions = {}) {
  const { onError, showToasts = true } = options;
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const handleError = useCallback(
    (error: Error, message: string) => {
      setError(error);
      if (showToasts) {
        toast.error(message);
      }
      if (onError) {
        onError(error);
      }
    },
    [onError, showToasts]
  );

  const loadConfigs = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const configs = await Config.readAll();
      return configs;
    } catch (err) {
      const error = err instanceof Error ? err : new Error('Failed to load configurations');
      handleError(error, 'Failed to load configurations');
      return {};
    } finally {
      setLoading(false);
    }
  }, [handleError]);

  const addConfig = useCallback(
    async (key: string, value: any) => {
      try {
        setLoading(true);
        setError(null);
        await Config.upsert(key, value);
        if (showToasts) {
          toast.success(`Successfully added configuration: ${key}`);
        }
        return true;
      } catch (err) {
        const error = err instanceof Error ? err : new Error('Failed to add configuration');
        handleError(error, `Failed to add configuration: ${key}`);
        return false;
      } finally {
        setLoading(false);
      }
    },
    [handleError, showToasts]
  );

  const removeConfig = useCallback(
    async (key: string) => {
      try {
        setLoading(true);
        setError(null);
        await Config.remove(key);
        if (showToasts) {
          toast.success(`Successfully removed configuration: ${key}`);
        }
        return true;
      } catch (err) {
        const error = err instanceof Error ? err : new Error('Failed to remove configuration');
        handleError(error, `Failed to remove configuration: ${key}`);
        return false;
      } finally {
        setLoading(false);
      }
    },
    [handleError, showToasts]
  );

  const readConfig = useCallback(
    async (key: string) => {
      try {
        setLoading(true);
        setError(null);
        const value = await Config.read(key);
        return value;
      } catch (err) {
        const error = err instanceof Error ? err : new Error('Failed to read configuration');
        handleError(error, `Failed to read configuration: ${key}`);
        return null;
      } finally {
        setLoading(false);
      }
    },
    [handleError]
  );

  return {
    loading,
    error,
    loadConfigs,
    addConfig,
    removeConfig,
    readConfig,
  };
}
