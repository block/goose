import { useCallback, useEffect, useRef, useState } from 'react';
import { AppEvents } from '../constants/events';

type StreamState = 'idle' | 'streaming' | 'error';

interface SessionStatus {
  streamState: StreamState;
  hasUnreadActivity: boolean;
}

/**
 * Simple hook to track session status for the sidebar.
 * Listens to session-status-update events from BaseChat components.
 */
export function useSidebarSessionStatus(activeSessionId: string | undefined) {
  const [statuses, setStatuses] = useState<Map<string, SessionStatus>>(new Map());
  const activeSessionIdRef = useRef(activeSessionId);

  // Keep ref in sync
  useEffect(() => {
    activeSessionIdRef.current = activeSessionId;
  }, [activeSessionId]);

  // Clear unread when active session changes
  useEffect(() => {
    if (activeSessionId) {
      setStatuses((prev) => {
        const status = prev.get(activeSessionId);
        if (status?.hasUnreadActivity) {
          const next = new Map(prev);
          next.set(activeSessionId, { ...status, hasUnreadActivity: false });
          return next;
        }
        return prev;
      });
    }
  }, [activeSessionId]);

  // Listen for status updates from BaseChat
  useEffect(() => {
    const handleStatusUpdate = (event: Event) => {
      const { sessionId, streamState } = (event as CustomEvent).detail;

      setStatuses((prev) => {
        const existing = prev.get(sessionId);
        const wasStreaming = existing?.streamState === 'streaming';
        const isNowIdle = streamState === 'idle';
        const isBackground = sessionId !== activeSessionIdRef.current;

        // Mark unread if streaming just finished in a background session
        const shouldMarkUnread = isBackground && wasStreaming && isNowIdle;

        const next = new Map(prev);
        next.set(sessionId, {
          streamState,
          hasUnreadActivity: existing?.hasUnreadActivity || shouldMarkUnread,
        });
        return next;
      });
    };

    window.addEventListener(AppEvents.SESSION_STATUS_UPDATE, handleStatusUpdate);
    return () => window.removeEventListener(AppEvents.SESSION_STATUS_UPDATE, handleStatusUpdate);
  }, []);

  const getSessionStatus = useCallback(
    (sessionId: string): SessionStatus | undefined => {
      return statuses.get(sessionId);
    },
    [statuses]
  );

  const clearUnread = useCallback((sessionId: string) => {
    setStatuses((prev) => {
      const status = prev.get(sessionId);
      if (status?.hasUnreadActivity) {
        const next = new Map(prev);
        next.set(sessionId, { ...status, hasUnreadActivity: false });
        return next;
      }
      return prev;
    });
  }, []);

  return { getSessionStatus, clearUnread };
}
