import { useEffect, useRef } from 'react';
import { Session } from '../api';
import { useSessionStreamContext } from '../contexts/SessionStreamContext';

interface UseSessionStreamOptions {
  enabled?: boolean;
  onUpdate?: (session: Session) => void;
  onError?: (error: string) => void;
}

interface UseSessionStreamResult {
  session: Session | undefined;
  isLoading: boolean;
  error: string | undefined;
  isConnected: boolean;
  refresh: () => Promise<void>;
}

/**
 * Hook to stream session updates from the server using Server-Sent Events (SSE).
 *
 * This hook provides real-time updates for a session, automatically reconnecting
 * on app refresh or connection loss. Uses React Context for global state management,
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
  const { getSessionState, refreshSession, registerStream, unregisterStream } =
    useSessionStreamContext();

  const onUpdateRef = useRef(onUpdate);
  const onErrorRef = useRef(onError);

  useEffect(() => {
    onUpdateRef.current = onUpdate;
    onErrorRef.current = onError;
  }, [onUpdate, onError]);

  // Register/unregister with the stream manager
  useEffect(() => {
    if (!enabled || !sessionId) {
      return;
    }

    registerStream(sessionId);

    return () => {
      unregisterStream(sessionId);
    };
  }, [sessionId, enabled, registerStream, unregisterStream]);

  // Get current state from context
  const state = sessionId
    ? getSessionState(sessionId)
    : {
        session: undefined,
        isLoading: false,
        error: undefined,
        isConnected: false,
      };

  // Call callbacks when state changes
  const prevSessionRef = useRef<Session | undefined>(undefined);
  const prevErrorRef = useRef<string | undefined>(undefined);

  useEffect(() => {
    if (state.session && state.session !== prevSessionRef.current && onUpdateRef.current) {
      onUpdateRef.current(state.session);
    }
    prevSessionRef.current = state.session;
  }, [state.session]);

  useEffect(() => {
    if (state.error && state.error !== prevErrorRef.current && onErrorRef.current) {
      onErrorRef.current(state.error);
    }
    prevErrorRef.current = state.error;
  }, [state.error]);

  const refresh = async (): Promise<void> => {
    if (!sessionId) return;
    await refreshSession(sessionId);
  };

  return {
    session: state.session,
    isLoading: state.isLoading,
    error: state.error,
    isConnected: state.isConnected,
    refresh,
  };
}

/**
 * Helper hook to just get session data from cache without streaming.
 * Useful for components that just need to read the session data that's
 * being updated by another component using useSessionStream.
 */
export function useSessionCache(sessionId: string | undefined): Session | undefined {
  const { getSessionState } = useSessionStreamContext();
  return sessionId ? getSessionState(sessionId).session : undefined;
}
