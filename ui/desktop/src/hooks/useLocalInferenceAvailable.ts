import { useState, useEffect } from 'react';
import { listLocalModels } from '../api';

export function useLocalInferenceAvailable() {
  const [isAvailable, setIsAvailable] = useState(true);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;

    (async () => {
      try {
        const result = await listLocalModels();
        if (cancelled) return;

        if (result.response?.status === 404) {
          setIsAvailable(false);
        }
      } catch {
        // Transient errors (network, 500, etc.) — assume available
      } finally {
        if (!cancelled) setIsLoading(false);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  return { isAvailable, isLoading };
}
