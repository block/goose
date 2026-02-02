import type { Transport } from './transport/transport.js';
import { HttpTransport } from './transport/http.js';
import { WebSocketTransport } from './transport/websocket.js';

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

export type TransportType = 'http' | 'websocket';

export interface AcpClientConfig {
  baseUrl: string;
  transport?: TransportType;
}

export class AcpClient {
  private transport: Transport;
  private baseUrl: string;
  private sessionId: string | null = null;
  private messageHandlers: MessageHandler[] = [];
  private errorHandlers: ErrorHandler[] = [];
  private requestHandlers: Map<string, RequestHandler> = new Map();
  private requestId = 0;
  private pendingRequests = new Map<string | number, {
    resolve: (result: unknown) => void;
    reject: (error: Error) => void;
  }>();

  constructor(config: AcpClientConfig) {
    this.baseUrl = config.baseUrl.replace(/\/$/, '');

    // Create appropriate transport
    const transportType = config.transport || 'http';
    this.transport = transportType === 'websocket'
      ? new WebSocketTransport()
      : new HttpTransport();

    // Set up message handling
    this.transport.onMessage((message) => this.handleMessage(message));
    this.transport.onError((error) => this.errorHandlers.forEach(h => h(error)));
  }

  getSessionId(): string | null {
    return this.sessionId;
  }

  isConnected(): boolean {
    return this.transport.isConnected();
  }

  async connect(): Promise<string> {
    this.sessionId = await this.transport.connect(this.baseUrl);
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

    // Create a promise that will be resolved when we get the response
    const promise = new Promise<T>((resolve, reject) => {
      this.pendingRequests.set(id, {
        resolve: resolve as (result: unknown) => void,
        reject
      });
    });

    // Send the request via transport
    const message: AcpMessage = { jsonrpc: '2.0', id, method, params };

    try {
      await this.transport.send(message);
    } catch (err) {
      this.pendingRequests.delete(id);
      throw err;
    }

    return promise;
  }

  async sendNotification(method: string, params?: unknown): Promise<void> {
    if (!this.sessionId) throw new Error('Not connected');
    await this.transport.send({ jsonrpc: '2.0', method, params });
  }

  // Send a response to a server-initiated request
  async sendResponse(requestId: string | number, result: unknown): Promise<void> {
    await this.transport.send({ jsonrpc: '2.0', id: requestId, result });
  }

  // Send an error response to a server-initiated request
  async sendErrorResponse(requestId: string | number, message: string): Promise<void> {
    await this.transport.send({
      jsonrpc: '2.0',
      id: requestId,
      error: { code: -32000, message }
    });
  }

  async disconnect(): Promise<void> {
    await this.transport.disconnect();
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
