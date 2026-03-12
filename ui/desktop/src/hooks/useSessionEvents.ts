import { useEffect, useRef, useState, useCallback } from 'react';
import { sessionEvents, type MessageEvent } from '../api';

/**
 * An SSE event with an optional request_id (added by the server at the
 * SSE framing layer, not part of the generated MessageEvent type).
 */
export type SessionEvent = MessageEvent & { request_id?: string };

type EventHandler = (event: SessionEvent) => void;

export function useSessionEvents(sessionId: string) {
  const listenersRef = useRef(new Map<string, Set<EventHandler>>());
  const abortRef = useRef<AbortController | null>(null);
  const [connected, setConnected] = useState(false);

  useEffect(() => {
    if (!sessionId) return;

    const abortController = new AbortController();
    abortRef.current = abortController;

    (async () => {
      try {
        const { stream } = await sessionEvents({
          path: { id: sessionId },
          signal: abortController.signal,
        });

        setConnected(true);

        for await (const event of stream) {
          if (abortController.signal.aborted) break;

          // The server adds request_id to the JSON at the SSE framing layer
          const sessionEvent = event as SessionEvent;
          const requestId = sessionEvent.request_id;

          if (requestId) {
            const handlers = listenersRef.current.get(requestId);
            if (handlers) {
              for (const handler of handlers) {
                handler(sessionEvent);
              }
            }
          }
        }
      } catch (error) {
        if (abortController.signal.aborted) return;
        console.warn('SSE connection ended:', error);
      } finally {
        setConnected(false);
      }
    })();

    return () => {
      abortController.abort();
      abortRef.current = null;
      setConnected(false);
    };
  }, [sessionId]);

  const addListener = useCallback((requestId: string, handler: EventHandler): (() => void) => {
    if (!listenersRef.current.has(requestId)) {
      listenersRef.current.set(requestId, new Set());
    }
    listenersRef.current.get(requestId)!.add(handler);

    return () => {
      const set = listenersRef.current.get(requestId);
      if (set) {
        set.delete(handler);
        if (set.size === 0) {
          listenersRef.current.delete(requestId);
        }
      }
    };
  }, []);

  return { connected, addListener };
}
