import React, {
  createContext,
  useContext,
  useRef,
  useState,
  useEffect,
  useCallback,
  ReactNode,
} from 'react';
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

interface SessionStreamState {
  session: Session | undefined;
  isLoading: boolean;
  error: string | undefined;
  isConnected: boolean;
}

interface SessionStreamContextValue {
  getSessionState: (sessionId: string) => SessionStreamState;
  refreshSession: (sessionId: string) => Promise<void>;
  registerStream: (sessionId: string) => void;
  unregisterStream: (sessionId: string) => void;
}

const SessionStreamContext = createContext<SessionStreamContextValue | undefined>(undefined);

/**
 * Provider that manages SSE streams for multiple sessions.
 * Each session ID gets its own stream, and all components using the same session ID
 * will see the same data.
 *
 * The stream behavior is controlled by the session's `in_use` flag:
 * - When subscribing to a session stream, the server immediately sends the current session state
 * - If `in_use = false`: The stream closes immediately after sending the initial state
 * - If `in_use = true`: The stream continues until the session is no longer in use
 * - When the agent finishes work and marks `in_use = false`, the stream sends a final update and closes
 *
 * This prevents unnecessary polling of idle sessions while still providing real-time updates
 * when the agent is actively working.
 */
export function SessionStreamProvider({ children }: { children: ReactNode }) {
  // Map of sessionId -> session state
  const [sessionStates, setSessionStates] = useState<Map<string, SessionStreamState>>(new Map());

  // Map of sessionId -> abort controller for cleanup
  const abortControllersRef = useRef<Map<string, AbortController>>(new Map());

  // Map of sessionId -> reconnect timeout
  const reconnectTimeoutsRef = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());

  // Map of sessionId -> active stream count (for reference counting)
  const activeStreamsRef = useRef<Map<string, number>>(new Map());

  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      // Cleanup all streams on unmount
      // eslint-disable-next-line react-hooks/exhaustive-deps
      const controllers = abortControllersRef.current;
      // eslint-disable-next-line react-hooks/exhaustive-deps
      const timeouts = reconnectTimeoutsRef.current;
      controllers.forEach((controller) => controller.abort());
      controllers.clear();
      timeouts.forEach((timeout) => clearTimeout(timeout));
      timeouts.clear();
    };
  }, []);

  const updateSessionState = useCallback(
    (sessionId: string, updates: Partial<SessionStreamState>) => {
      setSessionStates((prev) => {
        const newMap = new Map(prev);
        const current = newMap.get(sessionId) || {
          session: undefined,
          isLoading: false,
          error: undefined,
          isConnected: false,
        };
        newMap.set(sessionId, { ...current, ...updates });
        return newMap;
      });
    },
    []
  );

  // Store connect in a ref so it can recursively call itself
  const connectRef = useRef<((sessionId: string) => Promise<void>) | null>(null);

  const connect = useCallback(
    async (sessionId: string) => {
      try {
        // DEBUG LOGGING
        window.electron.logInfo(
          JSON.stringify({
            context: 'SessionStreamContext',
            event: 'connect_start',
            sessionId,
          })
        );

        // Clean up existing connection
        const existingController = abortControllersRef.current.get(sessionId);
        if (existingController) {
          console.log(
            `[SessionStreamContext] Aborting existing connection for session ${sessionId}`
          );
          window.electron.logInfo(
            JSON.stringify({
              context: 'SessionStreamContext',
              event: 'aborting_existing_connection',
              sessionId,
            })
          );
          existingController.abort();
          abortControllersRef.current.delete(sessionId);
        }

        const secretKey = await window.electron.getSecretKey();

        // Fetch initial session data
        try {
          const response = await fetch(getApiUrl(`/sessions/${sessionId}`), {
            headers: {
              'X-Secret-Key': secretKey,
            },
          });

          if (response.ok) {
            const initialSession: Session = await response.json();
            updateSessionState(sessionId, { session: initialSession, isLoading: false });
          } else {
            updateSessionState(sessionId, { isLoading: false });
          }
        } catch (err) {
          console.warn('Failed to fetch initial session data:', err);
          updateSessionState(sessionId, { isLoading: false });
        }

        // Create new abort controller for this stream
        const abortController = new AbortController();
        abortControllersRef.current.set(sessionId, abortController);

        // Use fetch with SSE streaming
        const response = await fetch(getApiUrl(`/sessions/${sessionId}/stream`), {
          method: 'GET',
          headers: {
            'X-Secret-Key': secretKey,
          },
          signal: abortController.signal,
        });

        if (!response.ok) {
          throw new Error(`HTTP ${response.status}`);
        }

        if (!response.body) {
          throw new Error('No response body');
        }

        updateSessionState(sessionId, { isConnected: true, error: undefined });

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
                  const sessionData = parsedEvent.session as Session;
                  updateSessionState(sessionId, { session: sessionData });
                  break;
                }

                case 'Error': {
                  updateSessionState(sessionId, { error: parsedEvent.error });
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

        // Stream ended normally (session no longer in use or completed)
        updateSessionState(sessionId, { isConnected: false });

        // Don't reconnect - the stream closed because the session is no longer in use
        // If the user starts a new interaction, a new stream will be created
      } catch (err) {
        if (err instanceof Error && err.name === 'AbortError') {
          // Expected when we abort the connection
          return;
        }

        console.error('SSE connection error:', err);
        updateSessionState(sessionId, {
          isConnected: false,
          error: err instanceof Error ? err.message : 'Connection error',
        });

        // Only reconnect on actual errors, not when stream closes normally
        // Clear existing reconnect timeout
        const existingTimeout = reconnectTimeoutsRef.current.get(sessionId);
        if (existingTimeout) {
          clearTimeout(existingTimeout);
        }

        // Schedule reconnect only for error cases
        const timeout = setTimeout(() => {
          if (mountedRef.current && activeStreamsRef.current.get(sessionId) && connectRef.current) {
            console.log('Attempting to reconnect to session stream after error...');
            connectRef.current(sessionId);
          }
        }, 3000);
        reconnectTimeoutsRef.current.set(sessionId, timeout);
      }
    },
    [updateSessionState]
  );

  connectRef.current = connect;

  const startStream = useCallback(
    (sessionId: string) => {
      const count = activeStreamsRef.current.get(sessionId) || 0;
      activeStreamsRef.current.set(sessionId, count + 1);

      // Only start the stream if this is the first subscriber
      if (count === 0) {
        updateSessionState(sessionId, { isLoading: true });
        connect(sessionId);
      }
    },
    [connect, updateSessionState]
  );

  const stopStream = useCallback(
    (sessionId: string) => {
      const count = activeStreamsRef.current.get(sessionId) || 0;
      const newCount = Math.max(0, count - 1);
      activeStreamsRef.current.set(sessionId, newCount);

      // Only cleanup if no more subscribers
      if (newCount === 0) {
        const controller = abortControllersRef.current.get(sessionId);
        if (controller) {
          controller.abort();
          abortControllersRef.current.delete(sessionId);
        }

        const timeout = reconnectTimeoutsRef.current.get(sessionId);
        if (timeout) {
          clearTimeout(timeout);
          reconnectTimeoutsRef.current.delete(sessionId);
        }

        updateSessionState(sessionId, { isConnected: false });
      }
    },
    [updateSessionState]
  );

  const refreshSession = useCallback(
    async (sessionId: string): Promise<void> => {
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
        updateSessionState(sessionId, { session: freshSession });
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : 'Failed to refresh session';
        updateSessionState(sessionId, { error: errorMsg });
        throw err;
      }
    },
    [updateSessionState]
  );

  const getSessionState = useCallback(
    (sessionId: string): SessionStreamState => {
      return (
        sessionStates.get(sessionId) || {
          session: undefined,
          isLoading: false,
          error: undefined,
          isConnected: false,
        }
      );
    },
    [sessionStates]
  );

  const value: SessionStreamContextValue = React.useMemo(
    () => ({
      getSessionState,
      refreshSession,
      registerStream: startStream,
      unregisterStream: stopStream,
    }),
    [getSessionState, refreshSession, startStream, stopStream]
  );

  return <SessionStreamContext.Provider value={value}>{children}</SessionStreamContext.Provider>;
}

export function useSessionStreamContext() {
  const context = useContext(SessionStreamContext);
  if (!context) {
    throw new Error('useSessionStreamContext must be used within SessionStreamProvider');
  }
  return context;
}
