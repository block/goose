import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import {
  listInstances,
  spawnInstance,
  cancelInstance as cancelInstanceApi,
  getInstanceResult,
} from '../lib/instances';
import type {
  InstanceResponse,
  InstanceResultResponse,
  SpawnInstanceRequest,
} from '../lib/instances';

const POLL_INTERVAL_ACTIVE_MS = 3000;
const POLL_INTERVAL_IDLE_MS = 15000;

export interface UseInstancesReturn {
  instances: InstanceResponse[];
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  spawn: (req: SpawnInstanceRequest) => Promise<InstanceResponse>;
  cancel: (id: string) => Promise<void>;
  getResult: (id: string) => Promise<InstanceResultResponse>;
  runningCount: number;
  completedCount: number;
  failedCount: number;
}

export function useInstances(enabled: boolean = true): UseInstancesReturn {
  const [instances, setInstances] = useState<InstanceResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const mountedRef = useRef(true);
  const pollingRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const refresh = useCallback(async () => {
    try {
      const data = await listInstances();
      if (mountedRef.current) {
        setInstances(data);
        setError(null);
      }
    } catch (e) {
      if (mountedRef.current) {
        setError(e instanceof Error ? e.message : 'Failed to load instances');
      }
    } finally {
      if (mountedRef.current) setLoading(false);
    }
  }, []);

  const spawn = useCallback(async (req: SpawnInstanceRequest): Promise<InstanceResponse> => {
    const result = await spawnInstance(req);
    // Optimistic: add the new instance to the list immediately
    if (mountedRef.current) {
      setInstances((prev) => [result, ...prev]);
    }
    return result;
  }, []);

  const cancel = useCallback(
    async (id: string): Promise<void> => {
      // Optimistic: mark as cancelled immediately
      if (mountedRef.current) {
        setInstances((prev) =>
          prev.map((inst) => (inst.id === id ? { ...inst, status: 'cancelled' as const } : inst))
        );
      }
      try {
        await cancelInstanceApi(id);
      } catch (e) {
        // Rollback on failure — refetch
        await refresh();
        throw e;
      }
    },
    [refresh]
  );

  const getResult = useCallback(async (id: string): Promise<InstanceResultResponse> => {
    return getInstanceResult(id);
  }, []);

  // Initial fetch
  useEffect(() => {
    mountedRef.current = true;
    if (enabled) {
      refresh();
    }
    return () => {
      mountedRef.current = false;
    };
  }, [enabled, refresh]);

  // Polling: faster when running instances exist
  const hasRunning = useMemo(() => instances.some((i) => i.status === 'running'), [instances]);

  useEffect(() => {
    if (!enabled) return;

    const interval = hasRunning ? POLL_INTERVAL_ACTIVE_MS : POLL_INTERVAL_IDLE_MS;

    pollingRef.current = setInterval(() => {
      refresh();
    }, interval);

    return () => {
      if (pollingRef.current) {
        clearInterval(pollingRef.current);
        pollingRef.current = null;
      }
    };
  }, [enabled, hasRunning, refresh]);

  const runningCount = useMemo(
    () => instances.filter((i) => i.status === 'running').length,
    [instances]
  );
  const completedCount = useMemo(
    () => instances.filter((i) => i.status === 'completed').length,
    [instances]
  );
  const failedCount = useMemo(
    () => instances.filter((i) => i.status === 'failed').length,
    [instances]
  );

  return {
    instances,
    loading,
    error,
    refresh,
    spawn,
    cancel,
    getResult,
    runningCount,
    completedCount,
    failedCount,
  };
}
