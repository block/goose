/**
 * Hand-written SSE client for long-lived session event streams.
 *
 * Unlike the generated SSE client (which is designed for request-response SSE),
 * this client maintains a persistent GET connection per session and supports
 * safe reconnection via Last-Event-ID.
 */

export interface SessionEvent {
  type: string;
  request_id?: string;
  [key: string]: unknown;
}

type EventHandler = (event: SessionEvent) => void;

const INITIAL_RECONNECT_DELAY = 1000;
const MAX_RECONNECT_DELAY = 30_000;

export class SessionSseClient {
  private reader: ReadableStreamDefaultReader<Uint8Array> | null = null;
  private lastEventId: string | undefined;
  private abortController: AbortController | null = null;
  private listeners = new Map<string, Set<EventHandler>>();
  private globalListeners = new Set<EventHandler>();
  private reconnectDelay = INITIAL_RECONNECT_DELAY;
  private closed = false;
  private _connected = false;
  private onConnectedChange?: (connected: boolean) => void;

  constructor(
    private baseUrl: string,
    private sessionId: string,
    private headers: Record<string, string> = {}
  ) {}

  get connected(): boolean {
    return this._connected;
  }

  private setConnected(value: boolean) {
    if (this._connected !== value) {
      this._connected = value;
      this.onConnectedChange?.(value);
    }
  }

  /**
   * Set a callback for connection state changes.
   */
  onConnectionChange(callback: (connected: boolean) => void): void {
    this.onConnectedChange = callback;
  }

  /**
   * Open the SSE connection. Auto-reconnects on failure.
   */
  connect(): void {
    this.closed = false;
    this.doConnect();
  }

  /**
   * Close the SSE connection permanently.
   */
  close(): void {
    this.closed = true;
    this.setConnected(false);
    this.abortController?.abort();
    this.abortController = null;
    this.reader = null;
    this.listeners.clear();
    this.globalListeners.clear();
  }

  /**
   * Register a listener for events with a specific request_id.
   * Returns an unsubscribe function.
   */
  addListener(requestId: string, handler: EventHandler): () => void {
    if (!this.listeners.has(requestId)) {
      this.listeners.set(requestId, new Set());
    }
    this.listeners.get(requestId)!.add(handler);

    return () => {
      const set = this.listeners.get(requestId);
      if (set) {
        set.delete(handler);
        if (set.size === 0) {
          this.listeners.delete(requestId);
        }
      }
    };
  }

  /**
   * Register a listener for ALL events (regardless of request_id).
   * Returns an unsubscribe function.
   */
  addGlobalListener(handler: EventHandler): () => void {
    this.globalListeners.add(handler);
    return () => {
      this.globalListeners.delete(handler);
    };
  }

  private async doConnect(): Promise<void> {
    if (this.closed) return;

    this.abortController = new AbortController();

    const url = `${this.baseUrl}/sessions/${this.sessionId}/events`;
    const headers: Record<string, string> = { ...this.headers };
    if (this.lastEventId) {
      headers['Last-Event-ID'] = this.lastEventId;
    }

    try {
      const response = await fetch(url, {
        method: 'GET',
        headers,
        signal: this.abortController.signal,
      });

      if (!response.ok) {
        throw new Error(`SSE connection failed: ${response.status}`);
      }

      if (!response.body) {
        throw new Error('SSE response has no body');
      }

      this.setConnected(true);
      this.reconnectDelay = INITIAL_RECONNECT_DELAY;

      await this.readStream(response.body);
    } catch (error) {
      if (this.closed) return;

      if (error instanceof DOMException && error.name === 'AbortError') {
        return;
      }

      this.setConnected(false);
      console.warn('SSE connection error, reconnecting...', error);

      // Exponential backoff
      await new Promise((resolve) => setTimeout(resolve, this.reconnectDelay));
      this.reconnectDelay = Math.min(this.reconnectDelay * 2, MAX_RECONNECT_DELAY);

      this.doConnect();
    }
  }

  private async readStream(body: ReadableStream<Uint8Array>): Promise<void> {
    const decoder = new TextDecoder();
    this.reader = body.getReader();

    let buffer = '';

    try {
      while (true) {
        const { done, value } = await this.reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });

        // Process complete SSE frames (double newline delimited)
        const frames = buffer.split('\n\n');
        // Last element is incomplete — keep it in the buffer
        buffer = frames.pop() || '';

        for (const frame of frames) {
          if (!frame.trim()) continue;
          this.processFrame(frame);
        }
      }
    } catch (error) {
      if (this.closed) return;
      throw error;
    } finally {
      this.reader = null;
    }

    // Stream ended — reconnect if not closed
    if (!this.closed) {
      this.setConnected(false);
      await new Promise((resolve) => setTimeout(resolve, this.reconnectDelay));
      this.doConnect();
    }
  }

  private processFrame(frame: string): void {
    let eventId: string | undefined;
    let data: string | undefined;

    for (const line of frame.split('\n')) {
      if (line.startsWith('id:')) {
        eventId = line.slice(3).trim();
      } else if (line.startsWith('data:')) {
        data = line.slice(5).trim();
      }
    }

    // Track the last event ID for reconnection
    if (eventId) {
      this.lastEventId = eventId;
    }

    if (!data) return;

    try {
      const event: SessionEvent = JSON.parse(data);

      // Dispatch to global listeners
      for (const handler of this.globalListeners) {
        handler(event);
      }

      // Dispatch to request-specific listeners
      if (event.request_id) {
        const handlers = this.listeners.get(event.request_id);
        if (handlers) {
          for (const handler of handlers) {
            handler(event);
          }
        }
      }
    } catch (e) {
      console.warn('Failed to parse SSE event data:', data, e);
    }
  }
}
