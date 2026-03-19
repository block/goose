// TODO: This context is a stopgap — BaseChat pushes stream state up, NavigationPanel reads it.
// The proper fix is for the sidebar to subscribe to the SSE bus per session and derive
// streaming/unread state directly, eliminating this context entirely.

import React, { createContext, useContext, useState, useCallback } from 'react';

export type StreamState = 'idle' | 'loading' | 'streaming' | 'error';

interface SessionStatus {
  streamState: StreamState;
  hasUnreadActivity: boolean;
}

interface SessionStatusContextValue {
  getSessionStatus: (sessionId: string) => SessionStatus | undefined;
  updateStreamState: (sessionId: string, streamState: StreamState) => void;
  clearUnread: (sessionId: string) => void;
}

const SessionStatusContext = createContext<SessionStatusContextValue | null>(null);

export function SessionStatusProvider({ children }: { children: React.ReactNode }) {
  const [statuses, setStatuses] = useState<Map<string, SessionStatus>>(new Map());

  const updateStreamState = useCallback((sessionId: string, streamState: StreamState) => {
    setStatuses((prev) => {
      const existing = prev.get(sessionId);
      const shouldMarkUnread = existing?.streamState === 'streaming' && streamState === 'idle';
      const next = new Map(prev);
      next.set(sessionId, {
        streamState,
        hasUnreadActivity: existing?.hasUnreadActivity || shouldMarkUnread,
      });
      return next;
    });
  }, []);

  const getSessionStatus = useCallback(
    (sessionId: string) => statuses.get(sessionId),
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

  return (
    <SessionStatusContext.Provider value={{ getSessionStatus, updateStreamState, clearUnread }}>
      {children}
    </SessionStatusContext.Provider>
  );
}

export function useSessionStatus() {
  const ctx = useContext(SessionStatusContext);
  if (!ctx) throw new Error('useSessionStatus must be used within SessionStatusProvider');
  return ctx;
}
