import { createContext, useContext, ReactNode } from 'react';
import { useSessionStatus, SessionStatus } from '../hooks/useSessionStatus';

interface SessionStatusContextType {
  sessionStatuses: SessionStatus[];
  getSessionStatus: (sessionId: string) => SessionStatus | undefined;
  getStreamingSessions: () => SessionStatus[];
  getUnreadSessions: () => SessionStatus[];
  setActiveSession: (sessionId: string) => void;
  markSessionActive: (sessionId: string) => void;
  trackSession: (sessionId: string) => void;
}

const SessionStatusContext = createContext<SessionStatusContextType | null>(null);

export function SessionStatusProvider({ children }: { children: ReactNode }) {
  const sessionStatus = useSessionStatus();

  return (
    <SessionStatusContext.Provider value={sessionStatus}>{children}</SessionStatusContext.Provider>
  );
}

export function useSessionStatusContext(): SessionStatusContextType {
  const context = useContext(SessionStatusContext);
  if (!context) {
    throw new Error('useSessionStatusContext must be used within SessionStatusProvider');
  }
  return context;
}
