import EventSource from 'eventsource';

export interface AcpMessage {
  jsonrpc: string;
  id?: string | number;
  method?: string;
  params?: unknown;
  result?: unknown;
  error?: { code: number; message: string; data?: unknown };
}

type MessageHandler = (message: AcpMessage) => void;
type ErrorHandler = (error: Error) => void;
type RequestHandler = (message: AcpMessage) => Promise<unknown>;

// ACP header constant
const HEADER_SESSION_ID = 'Acp-Session-Id';

export class AcpClient {
  private baseUrl: string;
  private sessionId: string | null = null;
  private eventSource: EventSource | null = null;
  private messageHandlers: MessageHandler[] = [];
  private errorHandlers: ErrorHandler[] = [];
  private requestHandlers: Map<string, RequestHandler> = new Map();
  private requestId = 0;
  private pendingRequests = new Map<string | number, {
    resolve: (result: unknown) => void;
    reject: (error: Error) => void;
  }>();

  constructor(config: { baseUrl: string }) {
    this.baseUrl = config.baseUrl.replace(/\/$/, '');
  }

  getSessionId(): string | null {
    return this.sessionId;
  }

  isConnected(): boolean {
    return this.sessionId !== null && this.eventSource !== null;
  }

  async connect(): Promise<string> {
    // Per ACP Streamable HTTP spec: send initialize request via POST /acp
    // The response is an SSE stream with the session ID in the Acp-Session-Id header
    const id = ++this.requestId;
    const initializeRequest = {
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
    const sessionId = response.headers.get(HEADER_SESSION_ID);
    if (!sessionId) {
      throw new Error('Server did not return session ID in Acp-Session-Id header');
    }
    this.sessionId = sessionId;

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
    this.eventSource = new EventSource(`${this.baseUrl}/acp`, {
      headers: {
        [HEADER_SESSION_ID]: this.sessionId,
        'Accept': 'text/event-stream'
      }
    });

    this.eventSource.onmessage = (event) => {
      try {
        this.handleMessage(JSON.parse(event.data));
      } catch {}
    };

    this.eventSource.onerror = () => {
      this.errorHandlers.forEach(h => h(new Error('SSE connection error')));
    };

    // Send initialized notification
    await this.sendNotification('notifications/initialized');

    return this.sessionId;
  }

  private async handleMessage(message: AcpMessage) {
    // Check if this is a response to a pending request
    if (message.id !== undefined && this.pendingRequests.has(message.id)) {
      const pending = this.pendingRequests.get(message.id)!;
      this.pendingRequests.delete(message.id);
      if (message.error) {
        pending.reject(new Error(message.error.message));
      } else {
        pending.resolve(message.result);
      }
      return;
    }

    // Check if this is a request from the server (has method and id)
    if (message.method && message.id !== undefined) {
      const handler = this.requestHandlers.get(message.method);
      if (handler) {
        try {
          const result = await handler(message);
          await this.sendResponse(message.id, result);
        } catch (err) {
          await this.sendErrorResponse(message.id, err instanceof Error ? err.message : 'Unknown error');
        }
      } else {
        // No handler - notify message handlers
        this.messageHandlers.forEach(h => h(message));
      }
      return;
    }

    // Otherwise it's a notification
    this.messageHandlers.forEach(h => h(message));
  }

  onMessage(handler: MessageHandler): () => void {
    this.messageHandlers.push(handler);
    return () => {
      const i = this.messageHandlers.indexOf(handler);
      if (i > -1) this.messageHandlers.splice(i, 1);
    };
  }

  onError(handler: ErrorHandler): () => void {
    this.errorHandlers.push(handler);
    return () => {
      const i = this.errorHandlers.indexOf(handler);
      if (i > -1) this.errorHandlers.splice(i, 1);
    };
  }

  // Register a handler for server-initiated requests
  onRequest(method: string, handler: RequestHandler): () => void {
    this.requestHandlers.set(method, handler);
    return () => {
      this.requestHandlers.delete(method);
    };
  }

  async sendRequest<T>(method: string, params?: unknown): Promise<T> {
    if (!this.sessionId) throw new Error('Not connected');

    const id = ++this.requestId;
    
    // Create a promise that will be resolved when we get the response via EventSource
    const promise = new Promise<T>((resolve, reject) => {
      this.pendingRequests.set(id, {
        resolve: resolve as (result: unknown) => void,
        reject
      });
    });

    // Send the request - the response will come via the EventSource
    const message = { jsonrpc: '2.0', id, method, params };
    
    const response = await fetch(`${this.baseUrl}/acp`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Accept': 'application/json, text/event-stream',
        [HEADER_SESSION_ID]: this.sessionId
      },
      body: JSON.stringify(message),
    });

    if (!response.ok) {
      this.pendingRequests.delete(id);
      throw new Error(`Failed to send request: ${response.statusText}`);
    }

    // Don't read from the POST response stream - the EventSource will receive all messages
    // Just consume and discard the response body to avoid connection issues
    response.body?.cancel();

    return promise;
  }

  async sendNotification(method: string, params?: unknown): Promise<void> {
    if (!this.sessionId) throw new Error('Not connected');
    await this.send({ jsonrpc: '2.0', method, params });
  }

  // Send a response to a server-initiated request
  async sendResponse(requestId: string | number, result: unknown): Promise<void> {
    await this.send({ jsonrpc: '2.0', id: requestId, result });
  }

  // Send an error response to a server-initiated request
  async sendErrorResponse(requestId: string | number, message: string): Promise<void> {
    await this.send({ 
      jsonrpc: '2.0', 
      id: requestId, 
      error: { code: -32000, message } 
    });
  }

  private async send(message: AcpMessage): Promise<void> {
    if (!this.sessionId) throw new Error('Not connected');

    const response = await fetch(`${this.baseUrl}/acp`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Accept': 'application/json, text/event-stream',
        [HEADER_SESSION_ID]: this.sessionId
      },
      body: JSON.stringify(message),
    });
    if (!response.ok) throw new Error(`Failed to send message: ${response.statusText}`);
  }

  async disconnect(): Promise<void> {
    if (this.sessionId) {
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
    this.eventSource?.close();
    this.eventSource = null;
    this.sessionId = null;
    this.pendingRequests.clear();
    this.requestHandlers.clear();
  }
}

// Session notification types for parsing ACP messages
export interface SessionNotificationParams {
  sessionId: string;
  update: {
    sessionUpdate: string;
    content?: { type: string; text?: string };
    id?: string;
    title?: string;
    status?: string;
    fields?: { 
      status?: string; 
      content?: unknown[];
      title?: string;
      rawInput?: unknown;
    };
  };
}

export interface PermissionRequestParams {
  sessionId: string;
  toolCallUpdate: {
    id: string;
    fields: {
      title?: string;
      rawInput?: unknown;
      content?: unknown[];
    };
  };
  options: Array<{
    id: string;
    label: string;
    kind: string;
  }>;
}

export function parseSessionUpdate(message: AcpMessage): {
  type: 'text' | 'thought' | 'tool_call' | 'tool_update' | 'permission_request' | 'unknown';
  sessionId?: string;
  data?: unknown;
} {
  if (message.method === 'session/update') {
    const params = message.params as SessionNotificationParams;
    const updateType = params.update?.sessionUpdate;
    
    switch (updateType) {
      case 'agent_message_chunk':
        return {
          type: 'text',
          sessionId: params.sessionId,
          data: params.update.content?.text || ''
        };
      case 'agent_thought_chunk':
        return {
          type: 'thought',
          sessionId: params.sessionId,
          data: params.update.content?.text || ''
        };
      case 'tool_call':
        return {
          type: 'tool_call',
          sessionId: params.sessionId,
          data: {
            id: params.update.id,
            title: params.update.title,
            status: params.update.status
          }
        };
      case 'tool_call_update':
        return {
          type: 'tool_update',
          sessionId: params.sessionId,
          data: {
            id: params.update.id,
            status: params.update.fields?.status,
            content: params.update.fields?.content
          }
        };
    }
  }
  
  if (message.method === 'request_permission') {
    const params = message.params as PermissionRequestParams;
    return {
      type: 'permission_request',
      sessionId: params.sessionId,
      data: params
    };
  }
  
  return { type: 'unknown' };
}
