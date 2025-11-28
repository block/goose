import { useEffect, useRef, useState, useCallback } from 'react';
import { WorkerCommand, WorkerResponse, WorkerConfig, SessionState } from '../workers/types';
import { Message } from '../api';

interface UseSessionWorkerReturn {
  isReady: boolean;
  initSession: (sessionId: string) => void;
  loadSession: (sessionId: string) => Promise<void>;
  startStream: (sessionId: string, userMessage: string, messages: Message[]) => Promise<void>;
  stopStream: (sessionId: string) => void;
  destroySession: (sessionId: string) => void;
  getSessionState: (sessionId: string) => Promise<SessionState | null>;
  getAllSessions: () => SessionState[];
  subscribeToSession: (
    sessionId: string,
    callback: (state: Partial<SessionState>) => void
  ) => () => void;
}

/**
 * Hook to interact with the session worker
 * Manages worker lifecycle and provides methods to control sessions
 */
type PendingRequest<T = unknown> = {
  resolve: (value: T) => void;
  reject: (error: unknown) => void;
};

export function useSessionWorker(config: WorkerConfig): UseSessionWorkerReturn {
  const workerRef = useRef<Worker | null>(null);
  const [isReady, setIsReady] = useState(false);
  const subscribersRef = useRef<Map<string, Set<(state: Partial<SessionState>) => void>>>(
    new Map()
  );
  const pendingRequestsRef = useRef<Map<string, PendingRequest>>(new Map());
  const allSessionsRef = useRef<SessionState[]>([]);

  useEffect(() => {
    const worker = new Worker(new URL('../workers/sessionWorker.ts', import.meta.url), {
      type: 'module',
    });

    workerRef.current = worker;

    worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
      const message = event.data;

      if (message.type === 'READY') {
        setIsReady(true);
        return;
      }

      if ('sessionId' in message) {
        const sessionId = message.sessionId;

        switch (message.type) {
          case 'SESSION_LOADED': {
            const request = pendingRequestsRef.current.get(`load-${sessionId}`);
            if (request) {
              request.resolve(message.state);
              pendingRequestsRef.current.delete(`load-${sessionId}`);
            }
            break;
          }

          case 'SESSION_UPDATE': {
            const subscribers = subscribersRef.current.get(sessionId);
            if (subscribers) {
              subscribers.forEach((callback) => callback(message.state));
            }
            break;
          }

          case 'STREAM_FINISHED': {
            const request = pendingRequestsRef.current.get(`stream-${sessionId}`);
            if (request) {
              if (message.error) {
                request.reject(new Error(message.error));
              } else {
                request.resolve(undefined);
              }
              pendingRequestsRef.current.delete(`stream-${sessionId}`);
            }
            break;
          }

          case 'ERROR': {
            const request =
              pendingRequestsRef.current.get(`stream-${sessionId}`) ||
              pendingRequestsRef.current.get(`load-${sessionId}`);
            if (request) {
              request.reject(new Error(message.error));
            }
            console.error(`Worker error for session ${sessionId}:`, message.error);
            break;
          }

          case 'SESSION_STATE': {
            const request = pendingRequestsRef.current.get(`state-${sessionId}`);
            if (request) {
              request.resolve(message.state);
              pendingRequestsRef.current.delete(`state-${sessionId}`);
            }
            break;
          }
        }
      } else if (message.type === 'ALL_SESSIONS') {
        allSessionsRef.current = message.sessions;
      }
    };

    worker.onerror = (error) => {
      console.error('Worker error:', error);
      setIsReady(false);
    };

    worker.postMessage({ type: 'INIT', config } as WorkerCommand);

    return () => {
      worker.terminate();
      workerRef.current = null;
      setIsReady(false);
    };
  }, [config]);

  const sendCommand = useCallback((command: WorkerCommand) => {
    if (!workerRef.current) {
      throw new Error('Worker not initialized');
    }
    workerRef.current.postMessage(command);
  }, []);

  const initSession = useCallback(
    (sessionId: string) => {
      sendCommand({ type: 'INIT_SESSION', sessionId });
    },
    [sendCommand]
  );

  const loadSession = useCallback(
    (sessionId: string): Promise<void> => {
      return new Promise<void>((resolve, reject) => {
        pendingRequestsRef.current.set(`load-${sessionId}`, {
          resolve: () => resolve(),
          reject,
        });
        sendCommand({ type: 'LOAD_SESSION', sessionId });
      });
    },
    [sendCommand]
  );

  const startStream = useCallback(
    (sessionId: string, userMessage: string, messages: Message[]): Promise<void> => {
      return new Promise<void>((resolve, reject) => {
        pendingRequestsRef.current.set(`stream-${sessionId}`, {
          resolve: () => resolve(),
          reject,
        });
        sendCommand({ type: 'START_STREAM', sessionId, userMessage, messages });
      });
    },
    [sendCommand]
  );

  const stopStream = useCallback(
    (sessionId: string) => {
      sendCommand({ type: 'STOP_STREAM', sessionId });
    },
    [sendCommand]
  );

  const destroySession = useCallback(
    (sessionId: string) => {
      sendCommand({ type: 'DESTROY_SESSION', sessionId });
      subscribersRef.current.delete(sessionId);
    },
    [sendCommand]
  );

  const getSessionState = useCallback(
    (sessionId: string): Promise<SessionState | null> => {
      return new Promise<SessionState | null>((resolve, reject) => {
        pendingRequestsRef.current.set(`state-${sessionId}`, {
          resolve: (value: unknown) => resolve(value as SessionState | null),
          reject,
        });
        sendCommand({ type: 'GET_SESSION_STATE', sessionId });
      });
    },
    [sendCommand]
  );

  const getAllSessions = useCallback((): SessionState[] => {
    sendCommand({ type: 'GET_ALL_SESSIONS' });
    return allSessionsRef.current;
  }, [sendCommand]);

  const subscribeToSession = useCallback(
    (sessionId: string, callback: (state: Partial<SessionState>) => void) => {
      if (!subscribersRef.current.has(sessionId)) {
        subscribersRef.current.set(sessionId, new Set());
      }
      subscribersRef.current.get(sessionId)!.add(callback);

      return () => {
        const subscribers = subscribersRef.current.get(sessionId);
        if (subscribers) {
          subscribers.delete(callback);
          if (subscribers.size === 0) {
            subscribersRef.current.delete(sessionId);
          }
        }
      };
    },
    []
  );

  return {
    isReady,
    initSession,
    loadSession,
    startStream,
    stopStream,
    destroySession,
    getSessionState,
    getAllSessions,
    subscribeToSession,
  };
}
