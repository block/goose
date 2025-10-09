import { useEffect, useRef, useState } from 'react';
import useSWR, { mutate as globalMutate } from 'swr';
import { Session } from '../api';
import { getApiUrl } from '../config';

const TextDecoder = globalThis.TextDecoder;

/**
 * Event types from the session SSE stream
 */
type SessionEvent =
  | { type: 'Session'; session: Session }
  | { type: 'Error'; error: string }
  | { type: 'Ping' };

interface UseSessionStreamOptions {
  /**
   * Whether to enable streaming. If false, will only fetch once.
   * @default true
   */
  enabled?: boolean;

  /**
   * Callback when session is updated
   */
  onUpdate?: (session: Session) => void;

  /**
   * Callback when an error occurs
   */
  onError?: (error: string) => void;
}

interface UseSessionStreamResult {
  /** Current session data */
  session: Session | undefined;

  /** Loading state - true on initial load */
  isLoading: boolean;

  /** Error message if any */
  error: string | undefined;

  /** Whether the stream is currently connected */
  isConnected: boolean;

  /** Manually refresh the session */
  refresh: () => Promise<void>;
}

/**
 * Hook to stream session updates from the server using Server-Sent Events (SSE).
 *
 * This hook provides real-time updates for a session, automatically reconnecting
 * on app refresh or connection loss. It integrates with SWR for global caching,
 * so all components using the same session ID will see the same data.
 *
 * @example
 * ```tsx
 * const { session, isLoading, isConnected } = useSessionStream('session-123', {
 *   onUpdate: (session) => console.log('Session updated:', session),
 *   onError: (error) => console.error('Stream error:', error),
 * });
 * ```
 */
export function useSessionStream(
  sessionId: string | undefined,
  options: UseSessionStreamOptions = {}
): UseSessionStreamResult {
  const { enabled = true, onUpdate, onError } = options;

  // Use SWR for global cache - all components with same sessionId will share this data
  const swrKey = sessionId ? `session-stream-${sessionId}` : null;
  const {
    data: session,
    error: swrError,
    mutate,
  } = useSWR<Session>(swrKey, null, {
    revalidateOnFocus: false,
    revalidateOnReconnect: false,
    dedupingInterval: 60000, // Cache for 1 minute
  });

  const [streamError, setStreamError] = useState<string | undefined>();
  const [isConnected, setIsConnected] = useState(false);
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const mountedRef = useRef(true);

  // Determine loading state
  const isLoading = !session && !swrError && !streamError;

  // Combine SWR error and stream error
  const error = streamError || (swrError ? String(swrError) : undefined);

  // Manual refresh function
  const refresh = async (): Promise<void> => {
    if (!sessionId) return;

    try {
      const secretKey = await window.electron.getSecretKey();
      const response = await fetch(getApiUrl(`/sessions/${sessionId}`), {
        headers: {
          'X-Secret-Key': secretKey,
        },
      });

      if (!response.ok) {
        throw new Error(`Failed to fetch session: ${response.status}`);
      }

      const freshSession: Session = await response.json();
      mutate(freshSession, false);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Failed to refresh session';
      setStreamError(errorMsg);
      throw err;
    }
  };

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  useEffect(() => {
    // Don't connect if disabled or no session ID
    if (!enabled || !sessionId) {
      return;
    }

    const abortControllerRef = { current: null as AbortController | null };

    const connect = async () => {
      try {
        // Clean up existing connection
        if (abortControllerRef.current) {
          abortControllerRef.current.abort();
          abortControllerRef.current = null;
        }

        const secretKey = await window.electron.getSecretKey();

        // Fetch initial session data to populate cache
        try {
          const response = await fetch(getApiUrl(`/sessions/${sessionId}`), {
            headers: {
              'X-Secret-Key': secretKey,
            },
          });

          if (response.ok) {
            const initialSession: Session = await response.json();
            mutate(initialSession, false);
          }
        } catch (err) {
          console.warn('Failed to fetch initial session data:', err);
        }

        // Create new abort controller for this stream
        abortControllerRef.current = new AbortController();

        // Use fetch with SSE streaming (same approach as /reply endpoint)
        const response = await fetch(getApiUrl(`/sessions/${sessionId}/stream`), {
          method: 'GET',
          headers: {
            'X-Secret-Key': secretKey,
          },
          signal: abortControllerRef.current.signal,
        });

        if (!response.ok) {
          throw new Error(`HTTP ${response.status}`);
        }

        if (!response.body) {
          throw new Error('No response body');
        }

        setIsConnected(true);
        setStreamError(undefined);

        const reader = response.body.getReader();
        const decoder = new TextDecoder();
        let buffer = '';

        while (true) {
          const { done, value } = await reader.read();
          if (done) break;

          buffer += decoder.decode(value, { stream: true });

          // Process complete SSE events
          const events = buffer.split('\n\n');
          buffer = events.pop() || ''; // Keep the last incomplete event in the buffer

          for (const event of events) {
            if (!event.startsWith('data: ')) continue;

            const data = event.slice(6); // Remove 'data: ' prefix
            if (data === '[DONE]') continue;

            try {
              const parsedEvent: SessionEvent = JSON.parse(data);

              if (!mountedRef.current) break;

              switch (parsedEvent.type) {
                case 'Session': {
                  // The session is boxed on the backend, but JSON serialization unwraps it
                  const sessionData = parsedEvent.session as Session;

                  // Update SWR cache - this will update all components using this session
                  mutate(sessionData, false);
                  globalMutate(`session-${sessionId}`, sessionData, false);

                  if (onUpdate) {
                    onUpdate(sessionData);
                  }
                  break;
                }

                case 'Error': {
                  setStreamError(parsedEvent.error);
                  if (onError) {
                    onError(parsedEvent.error);
                  }
                  break;
                }

                case 'Ping': {
                  // Heartbeat - connection is alive
                  break;
                }
              }
            } catch (err) {
              console.error('Failed to parse SSE event:', err);
            }
          }
        }

        // Stream ended normally
        setIsConnected(false);
      } catch (err) {
        if (err instanceof Error && err.name === 'AbortError') {
          // Expected when we abort the connection
          return;
        }

        console.error('SSE connection error:', err);
        setIsConnected(false);

        if (err instanceof Error) {
          setStreamError(err.message);
        }

        // Attempt to reconnect after a delay
        if (reconnectTimeoutRef.current) {
          clearTimeout(reconnectTimeoutRef.current);
        }

        reconnectTimeoutRef.current = setTimeout(() => {
          if (mountedRef.current && enabled) {
            console.log('Attempting to reconnect to session stream...');
            connect();
          }
        }, 3000); // Reconnect after 3 seconds
      }
    };

    // Initial connection
    connect();

    // Cleanup on unmount or when sessionId/enabled changes
    return () => {
      if (abortControllerRef.current) {
        abortControllerRef.current.abort();
        abortControllerRef.current = null;
      }
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current);
        reconnectTimeoutRef.current = null;
      }
      setIsConnected(false);
    };
  }, [sessionId, enabled, mutate, onUpdate, onError]);

  return {
    session,
    isLoading,
    error,
    isConnected,
    refresh,
  };
}

/**
 * Helper hook to just get session data from cache without streaming.
 * Useful for components that just need to read the session data that's
 * being updated by another component using useSessionStream.
 */
export function useSessionCache(sessionId: string | undefined): Session | undefined {
  const swrKey = sessionId ? `session-stream-${sessionId}` : null;
  const { data } = useSWR<Session>(swrKey, null, {
    revalidateOnFocus: false,
    revalidateOnReconnect: false,
  });
  return data;
}
