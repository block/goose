import React, { createContext, useContext, useState, useCallback } from 'react';
import { UserInput } from '../types/message';

interface ActiveSession {
  sessionId: string;
  initialMessage?: UserInput;
}

interface ActiveSessionsContextValue {
  activeSessions: ActiveSession[];
  addActiveSession: (sessionId: string, initialMessage?: UserInput) => void;
  clearInitialMessage: (sessionId: string) => void;
}

const MAX_ACTIVE_SESSIONS = 10;

const ActiveSessionsContext = createContext<ActiveSessionsContextValue | null>(null);

export function ActiveSessionsProvider({ children }: { children: React.ReactNode }) {
  const [activeSessions, setActiveSessions] = useState<ActiveSession[]>([]);

  const addActiveSession = useCallback((sessionId: string, initialMessage?: UserInput) => {
    setActiveSessions((prev) => {
      const existingIndex = prev.findIndex((s) => s.sessionId === sessionId);
      if (existingIndex !== -1) {
        const existing = prev[existingIndex];
        return [...prev.slice(0, existingIndex), ...prev.slice(existingIndex + 1), existing];
      }
      const updated = [...prev, { sessionId, initialMessage }];
      if (updated.length > MAX_ACTIVE_SESSIONS) {
        return updated.slice(updated.length - MAX_ACTIVE_SESSIONS);
      }
      return updated;
    });
  }, []);

  const clearInitialMessage = useCallback((sessionId: string) => {
    setActiveSessions((prev) =>
      prev.map((s) => (s.sessionId === sessionId ? { ...s, initialMessage: undefined } : s))
    );
  }, []);

  return (
    <ActiveSessionsContext.Provider value={{ activeSessions, addActiveSession, clearInitialMessage }}>
      {children}
    </ActiveSessionsContext.Provider>
  );
}

export function useActiveSessions() {
  const ctx = useContext(ActiveSessionsContext);
  if (!ctx) throw new Error('useActiveSessions must be used within ActiveSessionsProvider');
  return ctx;
}
