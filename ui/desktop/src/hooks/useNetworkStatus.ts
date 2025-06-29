import { useState, useEffect, useCallback } from 'react';

export type NetworkStatus = 'online' | 'offline' | 'degraded' | 'unknown';

export interface NetworkStatusInfo {
  status: NetworkStatus;
  description: string;
  lastChecked: Date;
  latency?: number;
}

interface UseNetworkStatusOptions {
  /** Interval for polling network status (in milliseconds) */
  pollingInterval?: number;
  /** Callback when network status changes */
  onStatusChange?: (status: NetworkStatusInfo) => void;
}

// Configuration constants
const DEFAULT_POLLING_INTERVAL = 30000; // 30 seconds

/**
 * Custom hook for monitoring network status
 *
 * Features:
 * - Detects online/offline state using browser APIs
 * - Provides instant visual feedback
 * - Simple and fast
 *
 * @param options Configuration options for the hook
 * @returns Network status information and control functions
 */
export function useNetworkStatus({
  pollingInterval = DEFAULT_POLLING_INTERVAL,
  onStatusChange,
}: UseNetworkStatusOptions = {}) {
  const [networkStatus, setNetworkStatus] = useState<NetworkStatusInfo>({
    status: window.navigator.onLine ? 'online' : 'offline',
    description: window.navigator.onLine ? 'Network is online' : 'Network is offline',
    lastChecked: new Date(),
  });

  const [isReconnecting, setIsReconnecting] = useState(false);
  const [reconnectAttempts, setReconnectAttempts] = useState(0);

  /**
   * Updates network status
   * Only triggers onChange callback when status actually changes
   */
  const updateStatus = useCallback(
    (status: NetworkStatus, description: string) => {
      const newStatus: NetworkStatusInfo = {
        status,
        description,
        lastChecked: new Date(),
      };

      setNetworkStatus((prev) => {
        if (prev.status !== newStatus.status) {
          onStatusChange?.(newStatus);
        }
        return newStatus;
      });
    },
    [onStatusChange]
  );

  /**
   * Simple network check using browser API
   */
  const checkNetworkStatus = useCallback(() => {
    if (window.navigator.onLine) {
      updateStatus('online', 'Network is online');
    } else {
      updateStatus('offline', 'Network is offline');
    }
  }, [updateStatus]);

  // Handle browser online/offline events
  useEffect(() => {
    const handleOnline = () => {
      updateStatus('online', 'Network is online');
      setIsReconnecting(false);
      setReconnectAttempts(0);
    };

    const handleOffline = () => {
      updateStatus('offline', 'Network is offline');
    };

    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);

    // Initial status check
    checkNetworkStatus();

    return () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
    };
  }, [checkNetworkStatus, updateStatus]);

  // Set up periodic checks (optional, browser events should handle most cases)
  useEffect(() => {
    const interval = setInterval(checkNetworkStatus, pollingInterval);

    return () => {
      clearInterval(interval);
    };
  }, [pollingInterval, checkNetworkStatus]);

  return {
    networkStatus,
    isReconnecting,
    reconnectAttempts,
    checkNetworkStatus,
  };
}
