/* global EventSource */
import { useState, useEffect, useRef, useCallback } from 'react';
import { createInstanceEventSourceUrl } from '../lib/instances';
import type { InstanceEvent } from '../lib/instances';

const MAX_EVENTS = 500;
const RECONNECT_BASE_MS = 1000;
const RECONNECT_MAX_MS = 30000;

export interface UseInstanceEventsReturn {
  events: InstanceEvent[];
  connected: boolean;
  error: string | null;
  clearEvents: () => void;
}

export function useInstanceEvents(
  instanceId: string | null,
  enabled: boolean = true
): UseInstanceEventsReturn {
  const [events, setEvents] = useState<InstanceEvent[]>([]);
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const eventSourceRef = useRef<EventSource | null>(null);
  const reconnectAttemptRef = useRef(0);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const mountedRef = useRef(true);

  const clearEvents = useCallback(() => {
    setEvents([]);
  }, []);

  const cleanup = useCallback(() => {
    if (eventSourceRef.current) {
      eventSourceRef.current.close();
      eventSourceRef.current = null;
    }
    if (reconnectTimerRef.current) {
      clearTimeout(reconnectTimerRef.current);
      reconnectTimerRef.current = null;
    }
  }, []);

  useEffect(() => {
    mountedRef.current = true;

    if (!instanceId || !enabled) {
      cleanup();
      setConnected(false);
      return;
    }

    const connect = () => {
      cleanup();

      const url = createInstanceEventSourceUrl(instanceId);
      const es = new EventSource(url);
      eventSourceRef.current = es;

      es.onopen = () => {
        if (mountedRef.current) {
          setConnected(true);
          setError(null);
          reconnectAttemptRef.current = 0;
        }
      };

      es.onmessage = (event) => {
        if (!mountedRef.current) return;

        try {
          const parsed = JSON.parse(event.data);
          const instanceEvent: InstanceEvent = {
            timestamp: Date.now(),
            type: parsed.type || 'message',
            data:
              typeof parsed.data === 'string' ? parsed.data : JSON.stringify(parsed.data || parsed),
          };

          setEvents((prev) => {
            const next = [...prev, instanceEvent];
            return next.length > MAX_EVENTS ? next.slice(-MAX_EVENTS) : next;
          });
        } catch {
          // Non-JSON event — store raw
          setEvents((prev) => {
            const next = [...prev, { timestamp: Date.now(), type: 'raw', data: event.data }];
            return next.length > MAX_EVENTS ? next.slice(-MAX_EVENTS) : next;
          });
        }
      };

      es.onerror = () => {
        if (!mountedRef.current) return;
        setConnected(false);

        // Exponential backoff reconnect
        const attempt = reconnectAttemptRef.current;
        const delay = Math.min(RECONNECT_BASE_MS * Math.pow(2, attempt), RECONNECT_MAX_MS);
        reconnectAttemptRef.current = attempt + 1;

        setError(`Connection lost. Reconnecting in ${Math.round(delay / 1000)}s...`);

        reconnectTimerRef.current = setTimeout(() => {
          if (mountedRef.current) {
            connect();
          }
        }, delay);
      };
    };

    connect();

    return () => {
      mountedRef.current = false;
      cleanup();
    };
  }, [instanceId, enabled, cleanup]);

  return { events, connected, error, clearEvents };
}
