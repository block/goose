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

export class AcpClient {
  private baseUrl: string;
  private sessionId: string | null = null;
  private eventSource: EventSource | null = null;
  private messageHandlers: MessageHandler[] = [];
  private errorHandlers: ErrorHandler[] = [];
  private requestId = 0;
  private pendingRequests = new Map<string | number, {
    resolve: (result: unknown) => void;
    reject: (error: Error) => void;
  }>();

  constructor(config: { baseUrl: string }) {
    this.baseUrl = config.baseUrl.replace(/\/$/, '');
  }

  async connect(): Promise<string> {
    const response = await fetch(`${this.baseUrl}/acp/session`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
    });

    if (!response.ok) {
      throw new Error(`Failed to create session: ${response.statusText}`);
    }

    const data = await response.json();
    this.sessionId = data.session_id;

    this.eventSource = new EventSource(`${this.baseUrl}${data.stream_url}`);
    this.eventSource.onmessage = (event) => {
      try {
        this.handleMessage(JSON.parse(event.data));
      } catch {}
    };
    this.eventSource.onerror = () => {
      this.errorHandlers.forEach(h => h(new Error('SSE connection error')));
    };

    return this.sessionId!;
  }

  private handleMessage(message: AcpMessage) {
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

  async sendRequest<T>(method: string, params?: unknown): Promise<T> {
    if (!this.sessionId) throw new Error('Not connected');

    const id = ++this.requestId;
    const promise = new Promise<T>((resolve, reject) => {
      this.pendingRequests.set(id, { resolve: resolve as (r: unknown) => void, reject });
    });

    await this.send({ jsonrpc: '2.0', id, method, params });
    return promise;
  }

  private async send(message: AcpMessage): Promise<void> {
    const response = await fetch(`${this.baseUrl}/acp/session/${this.sessionId}/message`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(message),
    });
    if (!response.ok) throw new Error(`Failed to send message: ${response.statusText}`);
  }

  disconnect(): void {
    this.eventSource?.close();
    this.eventSource = null;
    this.sessionId = null;
    this.pendingRequests.clear();
  }
}
