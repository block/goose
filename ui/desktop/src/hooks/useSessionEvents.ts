import { useEffect, useRef, useState, useCallback } from 'react';
import { SessionSseClient, type SessionEvent } from '../utils/sessionSseClient';

type EventHandler = (event: SessionEvent) => void;

export function useSessionEvents(sessionId: string) {
  const clientRef = useRef<SessionSseClient | null>(null);
  const [connected, setConnected] = useState(false);

  useEffect(() => {
    if (!sessionId) return;

    let cancelled = false;

    (async () => {
      const baseUrl = String(window.appConfig.get('GOOSE_API_HOST') || '');
      const secretKey = await window.electron.getSecretKey();

      if (cancelled) return;

      const headers: Record<string, string> = {};
      if (secretKey) {
        headers['X-Secret-Key'] = secretKey;
      }

      const client = new SessionSseClient(baseUrl, sessionId, headers);
      client.onConnectionChange(setConnected);
      client.connect();
      clientRef.current = client;
    })();

    return () => {
      cancelled = true;
      if (clientRef.current) {
        clientRef.current.close();
        clientRef.current = null;
      }
      setConnected(false);
    };
  }, [sessionId]);

  const addListener = useCallback(
    (requestId: string, handler: EventHandler): (() => void) => {
      const client = clientRef.current;
      if (!client) {
        return () => {};
      }
      return client.addListener(requestId, handler);
    },
    []
  );

  const addGlobalListener = useCallback(
    (handler: EventHandler): (() => void) => {
      const client = clientRef.current;
      if (!client) {
        return () => {};
      }
      return client.addGlobalListener(handler);
    },
    []
  );

  return { connected, addListener, addGlobalListener };
}
