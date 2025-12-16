import { useState, useCallback, useRef, useEffect } from 'react';

export type StreamState = 'idle' | 'loading' | 'streaming' | 'error';

export interface SessionStatus {
  sessionId: string;
  streamState: StreamState;
  messageCount: number;
  lastUpdated: number;
  hasUnreadActivity: boolean;
  lastMarkedActiveAt?: number;
}

/**
 * Hook to track status of all sessions
 * Provides streaming state, message counts, and unread indicators
 * Sessions update their own status via global events
 */
export function useSessionStatus() {
  const [sessionStatuses, setSessionStatuses] = useState<Map<string, SessionStatus>>(new Map());
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const activeSessionIdRef = useRef<string | null>(null);
  const trackedSessionsRef = useRef<Set<string>>(new Set());

  useEffect(() => {
    activeSessionIdRef.current = activeSessionId;
  }, [activeSessionId]);

  useEffect(() => {
    const handleSessionUpdate = (event: Event) => {
      const customEvent = event as CustomEvent;
      const { sessionId, streamState, messageCount } = customEvent.detail;

      setSessionStatuses((prev) => {
        const newMap = new Map(prev);
        const existing = newMap.get(sessionId) || {
          sessionId,
          streamState: 'idle' as StreamState,
          messageCount: 0,
          lastUpdated: Date.now(),
          hasUnreadActivity: false,
          lastMarkedActiveAt: undefined,
        };

        const wasStreaming = existing.streamState === 'streaming';
        const isNowStreaming = streamState === 'streaming';
        const isNowIdle = streamState === 'idle';
        const streamJustFinished = wasStreaming && isNowIdle;
        const streamJustStarted = !wasStreaming && isNowStreaming;
        const hasNewMessages = messageCount > existing.messageCount;
        const isBackgroundSession = sessionId !== activeSessionIdRef.current;

        // Don't set unread if session was just marked active (within last 2 seconds)
        const now = Date.now();
        const wasRecentlyMarkedActive =
          existing.lastMarkedActiveAt && now - existing.lastMarkedActiveAt < 2000;

        // Set unread if: background session AND NOT recently marked active AND (stream just finished OR has new messages OR stream just started)
        const shouldSetUnread =
          isBackgroundSession &&
          !wasRecentlyMarkedActive &&
          (streamJustFinished || streamJustStarted || hasNewMessages);

        const newStatus: SessionStatus = {
          ...existing,
          streamState: streamState || existing.streamState,
          messageCount: messageCount ?? existing.messageCount,
          lastUpdated: Date.now(),
          hasUnreadActivity: existing.hasUnreadActivity || shouldSetUnread,
          lastMarkedActiveAt: existing.lastMarkedActiveAt,
        };

        newMap.set(sessionId, newStatus);
        return newMap;
      });
    };

    // Listen for session updates from BaseChat components
    window.addEventListener('session-status-update', handleSessionUpdate);
    return () => {
      window.removeEventListener('session-status-update', handleSessionUpdate);
    };
  }, []);

  const trackSession = useCallback((sessionId: string) => {
    if (trackedSessionsRef.current.has(sessionId)) {
      return;
    }

    trackedSessionsRef.current.add(sessionId);

    setSessionStatuses((prev) => {
      const newMap = new Map(prev);
      if (!newMap.has(sessionId)) {
        newMap.set(sessionId, {
          sessionId,
          streamState: 'idle',
          messageCount: 0,
          lastUpdated: Date.now(),
          hasUnreadActivity: false,
        });
      }
      return newMap;
    });
  }, []);

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
