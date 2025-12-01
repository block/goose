import { createContext, useContext, ReactNode } from 'react';
import { useSessionWorker } from '../hooks/useSessionWorker';
import { WorkerConfig, SessionState } from '../workers/types';
import { Message, Session } from '../api';

interface SessionWorkerContextType {
  isReady: boolean;
  initSession: (sessionId: string) => void;
  loadSession: (sessionId: string) => Promise<SessionState>;
  startStream: (sessionId: string, userMessage: string, messages: Message[]) => Promise<void>;
  stopStream: (sessionId: string) => void;
  destroySession: (sessionId: string) => void;
  getSessionState: (sessionId: string) => Promise<SessionState | null>;
  updateSession: (sessionId: string, session: Session) => Promise<void>;
  subscribeToSession: (
    sessionId: string,
    callback: (state: Partial<SessionState>) => void
  ) => () => void;
  waitForReady: () => Promise<void>;
}

const SessionWorkerContext = createContext<SessionWorkerContextType | null>(null);

interface SessionWorkerProviderProps {
  children: ReactNode;
  config: WorkerConfig;
}

export function SessionWorkerProvider({ children, config }: SessionWorkerProviderProps) {
  const worker = useSessionWorker(config);

  return <SessionWorkerContext.Provider value={worker}>{children}</SessionWorkerContext.Provider>;
}

export function useSessionWorkerContext(): SessionWorkerContextType {
  const context = useContext(SessionWorkerContext);
  if (!context) {
    throw new Error('useSessionWorkerContext must be used within SessionWorkerProvider');
  }
  return context;
}
