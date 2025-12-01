import { useEffect, useRef, useState } from 'react';
import { WorkerConfig, SessionState } from '../workers/types';
import { SessionWorkerClient } from '../workers/SessionWorkerClient';
import { Message, Session } from '../api';

interface SessionWorkerInterface {
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

/**
 * Hook to interact with the session worker using the simplified client
 * Manages worker lifecycle and provides methods to control sessions
 */
export function useSessionWorker(config: WorkerConfig): SessionWorkerInterface {
  const clientRef = useRef<SessionWorkerClient | null>(null);
  const [isReady, setIsReady] = useState(false);

  // Initialize worker client on mount
  useEffect(() => {
    const client = new SessionWorkerClient(config);
    clientRef.current = client;

    client.waitForReady().then(() => {
      setIsReady(true);
    });

    return () => {
      client.terminate();
      clientRef.current = null;
      setIsReady(false);
    };
  }, [config]);

  if (!clientRef.current) {
    return {
      isReady: false,
      initSession: () => {},
      loadSession: async () => {
        throw new Error('Worker not initialized');
      },
      startStream: async () => {
        throw new Error('Worker not initialized');
      },
      stopStream: () => {},
      destroySession: () => {},
      getSessionState: async () => null,
      updateSession: async () => {
        throw new Error('Worker not initialized');
      },
      subscribeToSession: () => () => {},
      waitForReady: async () => {},
    };
  }

  const client = clientRef.current;

  return {
    isReady,
    initSession: (sessionId: string) => client.initSession(sessionId),
    loadSession: (sessionId: string) => client.loadSession(sessionId),
    startStream: (sessionId: string, userMessage: string, messages: Message[]) =>
      client.startStream(sessionId, userMessage, messages),
    stopStream: (sessionId: string) => client.stopStream(sessionId),
    destroySession: (sessionId: string) => client.destroySession(sessionId),
    getSessionState: (sessionId: string) => client.getSessionState(sessionId),
    updateSession: (sessionId: string, session: Session) =>
      client.updateSession(sessionId, session),
    subscribeToSession: (sessionId: string, callback: (state: Partial<SessionState>) => void) =>
      client.subscribeToSession(sessionId, callback),
    waitForReady: () => client.waitForReady(),
  };
}
