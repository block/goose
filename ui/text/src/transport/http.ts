import EventSource from 'eventsource';
import type { AcpMessage } from '../client.js';
import { HEADER_SESSION_ID, type Transport, type MessageHandler, type ErrorHandler } from './transport.js';

/**
 * HTTP transport implementation for ACP.
 * Uses POST for client-to-server messages and EventSource (SSE) for server-to-client messages.
 */
export class HttpTransport implements Transport {
  private baseUrl: string | null = null;
  private sessionId: string | null = null;
  private eventSource: EventSource | null = null;
  private messageHandlers: Set<MessageHandler> = new Set();
  private errorHandlers: Set<ErrorHandler> = new Set();
  private connected: boolean = false;
  private requestId = 0;

  async connect(baseUrl: string, sessionId?: string): Promise<string> {
    this.baseUrl = baseUrl.replace(/\/$/, '');

    if (sessionId) {
      // Reconnection with existing session
      this.sessionId = sessionId;
      this.setupEventSource();
      return sessionId;
    }

    // Per ACP Streamable HTTP spec: send initialize request via POST /acp
    // The response is an SSE stream with the session ID in the Acp-Session-Id header
    const id = ++this.requestId;
    const initializeRequest: AcpMessage = {
      jsonrpc: '2.0',
      id,
      method: 'initialize',
      params: {
        protocolVersion: '2024-11-05',
        capabilities: {},
        clientInfo: {
          name: 'goose-tui',
          version: '1.0.0'
        }
      }
    };

    const response = await fetch(`${this.baseUrl}/acp`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Accept': 'application/json, text/event-stream'
      },
      body: JSON.stringify(initializeRequest)
    });

    if (!response.ok) {
      throw new Error(`Failed to initialize session: ${response.statusText}`);
    }

    // Get session ID from response header
    const sessionIdHeader = response.headers.get(HEADER_SESSION_ID);
    if (!sessionIdHeader) {
      throw new Error('Server did not return session ID in Acp-Session-Id header');
    }
    this.sessionId = sessionIdHeader;

    // The response body is an SSE stream - we need to read the initialize response
    const reader = response.body?.getReader();
    if (reader) {
      const decoder = new TextDecoder();
      let buffer = '';
      let initializeResponseReceived = false;

      while (!initializeResponseReceived) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split('\n');
        buffer = lines.pop() || '';

        for (const line of lines) {
          if (line.startsWith('data: ')) {
            const data = line.slice(6);
            if (data) {
              try {
                const message = JSON.parse(data);
                if (message.id === id) {
                  if (message.error) {
                    throw new Error(message.error.message);
                  }
                  initializeResponseReceived = true;
                  break;
                }
              } catch (e) {
                if (e instanceof SyntaxError) {
                  // Ignore JSON parse errors
                } else {
                  throw e;
                }
              }
            }
          }
        }
      }

      // Cancel the reader - we'll use EventSource for ongoing communication
      await reader.cancel();
    }

    // Set up EventSource for server-to-client messages using GET /acp
    this.setupEventSource();

    // Send initialized notification
    await this.send({
      jsonrpc: '2.0',
      method: 'notifications/initialized'
    });

    return this.sessionId;
  }

  private setupEventSource(): void {
    if (!this.baseUrl || !this.sessionId) return;

    this.eventSource = new EventSource(`${this.baseUrl}/acp`, {
      headers: {
        [HEADER_SESSION_ID]: this.sessionId,
        'Accept': 'text/event-stream'
      }
    });

    this.eventSource.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data);
        this.messageHandlers.forEach(h => h(message));
      } catch (err) {
        // Ignore JSON parse errors
      }
    };

    this.eventSource.onerror = () => {
      this.errorHandlers.forEach(h => h(new Error('SSE connection error')));
    };

    this.connected = true;
  }

  async send(message: AcpMessage): Promise<void> {
    if (!this.baseUrl || !this.sessionId) {
      throw new Error('Not connected');
    }

    const response = await fetch(`${this.baseUrl}/acp`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Accept': 'application/json, text/event-stream',
        [HEADER_SESSION_ID]: this.sessionId
      },
      body: JSON.stringify(message)
    });

    if (!response.ok) {
      throw new Error(`Failed to send message: ${response.statusText}`);
    }

    // Don't read from the POST response stream - the EventSource will receive all messages
    // Just consume and discard the response body to avoid connection issues
    response.body?.cancel();
  }

  async disconnect(): Promise<void> {
    if (this.eventSource) {
      this.eventSource.close();
      this.eventSource = null;
    }

    if (this.baseUrl && this.sessionId) {
      try {
        // Per ACP spec: DELETE /acp to terminate session
        await fetch(`${this.baseUrl}/acp`, {
          method: 'DELETE',
          headers: {
            [HEADER_SESSION_ID]: this.sessionId
          }
        });
      } catch {
        // Ignore errors during disconnect
      }
    }

    this.connected = false;
    this.sessionId = null;
    this.baseUrl = null;
  }

  onMessage(handler: MessageHandler): () => void {
    this.messageHandlers.add(handler);
    return () => this.messageHandlers.delete(handler);
  }

  onError(handler: ErrorHandler): () => void {
    this.errorHandlers.add(handler);
    return () => this.errorHandlers.delete(handler);
  }

  isConnected(): boolean {
    return this.connected && this.sessionId !== null;
  }
}
