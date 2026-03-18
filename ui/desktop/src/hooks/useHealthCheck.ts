import { useEffect, useRef, useCallback } from 'react';
import { status } from '../api';
import { toastError } from '../toasts';

/**
 * Interval between health checks in milliseconds.
 * We check every 30 seconds when the session is idle.
 */
const HEALTH_CHECK_INTERVAL_MS = 30_000;

/**
 * Number of consecutive failures before showing a warning to the user.
 * A single missed ping could be a transient blip — we wait for 3 in a row.
 */
const FAILURE_THRESHOLD = 3;

interface UseHealthCheckProps {
  /** Whether the health check should be active (e.g., session is loaded and idle) */
  enabled: boolean;
  /** Callback invoked when the backend is confirmed unreachable after FAILURE_THRESHOLD misses */
  onUnreachable?: () => void;
  /** Callback invoked when the backend recovers after being unreachable */
  onRecovered?: () => void;
}

/**
 * Periodically pings the goosed `/status` endpoint to detect backend
 * unavailability between user messages. This catches:
 * - goosed process crashes
 * - Network/TCP connection silently dying (e.g., after macOS sleep/wake)
 * - Server restarts
 *
 * When the backend becomes unreachable, a toast is shown to the user
 * and an optional callback is fired so the parent can attempt reconnection.
 */
export function useHealthCheck({
  enabled,
  onUnreachable,
  onRecovered,
}: UseHealthCheckProps): void {
  const consecutiveFailuresRef = useRef(0);
  const wasUnreachableRef = useRef(false);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const checkHealth = useCallback(async () => {
    try {
      await status({ throwOnError: true });

      // Backend is reachable — reset failure counter
      if (consecutiveFailuresRef.current > 0) {
        consecutiveFailuresRef.current = 0;
      }

      // If we were previously unreachable, notify recovery
      if (wasUnreachableRef.current) {
        wasUnreachableRef.current = false;
        onRecovered?.();
      }
    } catch {
      consecutiveFailuresRef.current += 1;

      if (
        consecutiveFailuresRef.current >= FAILURE_THRESHOLD &&
        !wasUnreachableRef.current
      ) {
        wasUnreachableRef.current = true;
        toastError({
          title: 'Backend unreachable',
          msg: 'Goose cannot reach the server. Your session will be restored automatically when the connection recovers.',
        });
        onUnreachable?.();
      }
    }
  }, [onUnreachable, onRecovered]);

  useEffect(() => {
    if (!enabled) {
      // Clear any running interval when disabled
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
      return;
    }

    // Start periodic health checks
    intervalRef.current = setInterval(checkHealth, HEALTH_CHECK_INTERVAL_MS);

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
    };
  }, [enabled, checkHealth]);
}
