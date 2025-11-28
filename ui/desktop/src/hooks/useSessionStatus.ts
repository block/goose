import { useEffect, useState, useCallback, useRef } from 'react';
import { useSessionWorkerContext } from '../contexts/SessionWorkerContext';
import { StreamState } from '../workers/types';

export interface SessionStatus {
  sessionId: string;
  streamState: StreamState;
  messageCount: number;
  lastUpdated: number;
  hasUnreadActivity: boolean;
  lastMarkedActiveAt?: number;
}

/**
 * Hook to track status of all sessions in the worker
 * Provides streaming state, message counts, and unread indicators
 */
export function useSessionStatus() {
  const worker = useSessionWorkerContext();
  const [sessionStatuses, setSessionStatuses] = useState<Map<string, SessionStatus>>(new Map());
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const activeSessionIdRef = useRef<string | null>(null);
  const trackedSessionsRef = useRef<Set<string>>(new Set());

  useEffect(() => {
    activeSessionIdRef.current = activeSessionId;
  }, [activeSessionId]);

  const trackSession = useCallback(
    (sessionId: string) => {
      if (trackedSessionsRef.current.has(sessionId)) {
        return;
      }

      trackedSessionsRef.current.add(sessionId);

      setSessionStatuses((prev) => {
        const newMap = new Map(prev);
        newMap.set(sessionId, {
          sessionId,
          streamState: 'idle',
          messageCount: 0,
          lastUpdated: Date.now(),
          hasUnreadActivity: false,
        });
        return newMap;
      });

      if (worker.isReady) {
        worker.subscribeToSession(sessionId, (update) => {
          setSessionStatuses((prev) => {
            const newMap = new Map(prev);
            const existing = newMap.get(sessionId) || {
              sessionId,
              streamState: 'idle' as StreamState,
              messageCount: 0,
              lastUpdated: Date.now(),
              hasUnreadActivity: false,
            };

            const wasStreaming = existing.streamState === 'streaming';
            const isNowStreaming = update.streamState === 'streaming';
            const isNowIdle = update.streamState === 'idle';
            const streamJustFinished = wasStreaming && isNowIdle;
            const streamJustStarted = !wasStreaming && isNowStreaming;
            const hasNewMessages =
              update.messages !== undefined && update.messages.length > existing.messageCount;
            const isBackgroundSession = sessionId !== activeSessionIdRef.current;

            // Don't set unread if session was just marked active (within last 2 seconds)
            // This prevents race conditions where updates arrive after marking active
            const now = Date.now();
            const wasRecentlyMarkedActive =
              existing.lastMarkedActiveAt && now - existing.lastMarkedActiveAt < 2000;

            // Calculate if we should SET unread (but never clear it here - only markSessionActive clears)
            // Set unread if: background session AND NOT recently marked active AND (stream just finished OR has new messages OR stream just started)
            const shouldSetUnread =
              isBackgroundSession &&
              !wasRecentlyMarkedActive &&
              (streamJustFinished || streamJustStarted || hasNewMessages);

            const newStatus: SessionStatus = {
              ...existing,
              streamState: update.streamState || existing.streamState,
              messageCount: update.messages?.length ?? existing.messageCount,
              lastUpdated: Date.now(),
              hasUnreadActivity: existing.hasUnreadActivity || shouldSetUnread,
              lastMarkedActiveAt: existing.lastMarkedActiveAt,
            };

            newMap.set(sessionId, newStatus);
            return newMap;
          });
        });
      }
    },
    [worker]
  );

  const setActiveSession = useCallback(
    (sessionId: string) => {
      activeSessionIdRef.current = sessionId;
      setActiveSessionId(sessionId);
      trackSession(sessionId);
    },
    [trackSession]
  );

  // Mark a session as active (user is viewing it) - clears unread indicator
  const markSessionActive = useCallback(
    (sessionId: string) => {
      activeSessionIdRef.current = sessionId;
      setActiveSessionId(sessionId);
      if (!trackedSessionsRef.current.has(sessionId)) {
        trackSession(sessionId);
      }

      const now = Date.now();
      setSessionStatuses((prev) => {
        const status = prev.get(sessionId);
        if (!status) {
          const newMap = new Map(prev);
          newMap.set(sessionId, {
            sessionId,
            streamState: 'idle',
            messageCount: 0,
            lastUpdated: now,
            hasUnreadActivity: false,
            lastMarkedActiveAt: now,
          });
          return newMap;
        }

        const newMap = new Map(prev);
        newMap.set(sessionId, {
          ...status,
          hasUnreadActivity: false,
          lastMarkedActiveAt: now,
        });
        return newMap;
      });
    },
    [trackSession]
  );

  useEffect(() => {
    if (!worker.isReady) return;
    const subscriptions = new Map<string, () => void>();
    return () => {
      subscriptions.forEach((unsubscribe) => unsubscribe());
      subscriptions.clear();
    };
  }, [worker.isReady, worker, activeSessionId]);

  const getSessionStatus = useCallback(
    (sessionId: string): SessionStatus | undefined => {
      return sessionStatuses.get(sessionId);
    },
    [sessionStatuses]
  );

  const getStreamingSessions = useCallback((): SessionStatus[] => {
    return Array.from(sessionStatuses.values()).filter(
      (status) => status.streamState === 'streaming'
    );
  }, [sessionStatuses]);

  const getUnreadSessions = useCallback((): SessionStatus[] => {
    return Array.from(sessionStatuses.values()).filter((status) => status.hasUnreadActivity);
  }, [sessionStatuses]);

  return {
    sessionStatuses: Array.from(sessionStatuses.values()),
    getSessionStatus,
    getStreamingSessions,
    getUnreadSessions,
    setActiveSession,
    markSessionActive,
    trackSession,
  };
}
