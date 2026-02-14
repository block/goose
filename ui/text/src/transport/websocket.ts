import WebSocket from 'ws';
import type { IncomingMessage } from 'http';
import type { AcpMessage } from '../client.js';
import { HEADER_SESSION_ID, type Transport, type MessageHandler, type ErrorHandler } from './transport.js';

/**
 * WebSocket transport implementation for ACP.
 * Uses a single persistent WebSocket connection for bidirectional communication.
 */
export class WebSocketTransport implements Transport {
  private ws: WebSocket | null = null;
  private sessionId: string | null = null;
  private messageHandlers: Set<MessageHandler> = new Set();
  private errorHandlers: Set<ErrorHandler> = new Set();
  private pendingMessages: AcpMessage[] = [];
  private connected: boolean = false;
  private initialized: boolean = false;
  private requestId = 0;

  async connect(baseUrl: string, sessionId?: string): Promise<string> {
    // Convert HTTP URL to WebSocket URL
    const wsUrl = baseUrl.replace(/^http/, 'ws').replace(/\/$/, '') + '/acp';

    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(wsUrl);

      // Capture session ID from HTTP upgrade response headers
      this.ws.on('upgrade', (response: IncomingMessage) => {
        const sessionIdHeader = response.headers[HEADER_SESSION_ID.toLowerCase()];
        if (!sessionIdHeader) {
          reject(new Error('Server did not return session ID in Acp-Session-Id header'));
          this.ws?.close();
          return;
        }
        this.sessionId = Array.isArray(sessionIdHeader) ? sessionIdHeader[0] : sessionIdHeader;
      });

      this.ws.on('open', async () => {
        this.connected = true;

        // Send initialize message
        const id = ++this.requestId;
        const initMessage: AcpMessage = {
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

        this.ws!.send(JSON.stringify(initMessage));

        // Wait for initialize response
        const responseHandler = (message: AcpMessage) => {
          if (message.id === id) {
            if ('result' in message && message.result) {
              // Session ID was already captured from HTTP upgrade response headers
              this.initialized = true;

              // Send initialized notification
              this.send({
                jsonrpc: '2.0',
                method: 'notifications/initialized'
              }).catch(err => {
                this.errorHandlers.forEach(h => h(err));
              });

              // Send any queued messages
              this.pendingMessages.forEach(msg => {
                this.send(msg).catch(err => {
                  this.errorHandlers.forEach(h => h(err));
                });
              });
              this.pendingMessages = [];

              this.messageHandlers.delete(responseHandler);

              if (!this.sessionId) {
                reject(new Error('Session ID not set'));
                return;
              }

              resolve(this.sessionId);
            } else if ('error' in message && message.error) {
              const errorMsg = typeof message.error === 'object' && message.error !== null
                ? (message.error as any).message || 'Init failed'
                : 'Init failed';
              reject(new Error(errorMsg));
            }
          }
        };

        this.messageHandlers.add(responseHandler);
      });

      this.ws.on('message', (data) => {
        try {
          const message = JSON.parse(data.toString()) as AcpMessage;
          this.messageHandlers.forEach(h => h(message));
        } catch (err) {
          this.errorHandlers.forEach(h => h(err instanceof Error ? err : new Error(String(err))));
        }
      });

      this.ws.on('error', (err) => {
        this.errorHandlers.forEach(h => h(err));
        if (!this.connected) {
          reject(err);
        }
      });

      this.ws.on('close', () => {
        this.connected = false;
        this.initialized = false;
      });
    });
  }

  async send(message: AcpMessage): Promise<void> {
    if (!this.ws || !this.connected) {
      throw new Error('WebSocket not connected');
    }

    if (!this.initialized && message.method !== 'initialize' && message.method !== 'notifications/initialized') {
      // Queue messages until initialized
      this.pendingMessages.push(message);
      return;
    }

    return new Promise((resolve, reject) => {
      this.ws!.send(JSON.stringify(message), (err) => {
        if (err) {
          reject(err);
        } else {
          resolve();
        }
      });
    });
  }

  async disconnect(): Promise<void> {
    if (this.ws) {
      this.ws.close(1000, 'Normal closure');
      this.ws = null;
    }
    this.connected = false;
    this.initialized = false;
    this.sessionId = null;
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
    return this.connected && this.initialized;
  }
}
