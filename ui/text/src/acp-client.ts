/**
 * ACP Client using the official @agentclientprotocol/sdk
 * 
 * This module provides a client implementation that uses the SDK's
 * ClientSideConnection with a custom HTTP/SSE stream adapter.
 */

import { 
  ClientSideConnection,
  type Client,
  type Agent,
  type RequestPermissionRequest,
  type RequestPermissionResponse,
  type SessionNotification,
  type ReadTextFileRequest,
  type ReadTextFileResponse,
  type WriteTextFileRequest,
  type WriteTextFileResponse,
  PROTOCOL_VERSION
} from '@agentclientprotocol/sdk';
import type { Stream } from '@agentclientprotocol/sdk';
import type { AnyMessage } from '@agentclientprotocol/sdk';
import EventSource from 'eventsource';

// Re-export types for convenience
export type { 
  SessionNotification,
  RequestPermissionRequest,
  RequestPermissionResponse,
  Agent
} from '@agentclientprotocol/sdk';

export { PROTOCOL_VERSION } from '@agentclientprotocol/sdk';

/**
 * Options for creating an ACP client
 */
export interface AcpClientOptions {
  /** Base URL of the ACP server */
  serverUrl: string;
  /** Custom headers for requests */
  headers?: Record<string, string>;
}

/**
 * Client handler callbacks
 */
export interface ClientHandlers {
  /** Called when the agent requests permission for a tool call */
  onPermissionRequest?: (request: RequestPermissionRequest) => Promise<RequestPermissionResponse>;
  /** Called when the agent sends a session update */
  onSessionUpdate?: (notification: SessionNotification) => void;
  /** Called when the agent requests to read a file */
  onReadTextFile?: (request: ReadTextFileRequest) => Promise<ReadTextFileResponse>;
  /** Called when the agent requests to write a file */
  onWriteTextFile?: (request: WriteTextFileRequest) => Promise<WriteTextFileResponse>;
}

/**
 * ACP Client that uses the official SDK with HTTP/SSE transport
 */
export class SdkAcpClient {
  private serverUrl: string;
  private headers: Record<string, string>;
  private connection: ClientSideConnection | null = null;
  private httpStream: HttpSseStream | null = null;
  private sessionId: string | null = null;
  private handlers: ClientHandlers;
  private agent: Agent | null = null;

  constructor(options: AcpClientOptions, handlers: ClientHandlers = {}) {
    this.serverUrl = options.serverUrl.replace(/\/$/, '');
    this.headers = options.headers || {};
    this.handlers = handlers;
  }

  /**
   * Connect to the ACP server and initialize a session
   */
  async connect(): Promise<string> {
    // Create the HTTP/SSE stream
    this.httpStream = new HttpSseStream({
      serverUrl: this.serverUrl,
      headers: this.headers
    });

    // Create the client handler
    const createClient = (agent: Agent): Client => {
      this.agent = agent;
      return {
        requestPermission: async (request: RequestPermissionRequest): Promise<RequestPermissionResponse> => {
          if (this.handlers.onPermissionRequest) {
            return this.handlers.onPermissionRequest(request);
          }
          // Default: cancelled (user didn't respond)
          return { outcome: { outcome: 'cancelled' } } as RequestPermissionResponse;
        },
        sessionUpdate: async (notification: SessionNotification): Promise<void> => {
          if (this.handlers.onSessionUpdate) {
            this.handlers.onSessionUpdate(notification);
          }
        },
        readTextFile: async (request: ReadTextFileRequest): Promise<ReadTextFileResponse> => {
          if (this.handlers.onReadTextFile) {
            return this.handlers.onReadTextFile(request);
          }
          throw new Error('File reading not supported');
        },
        writeTextFile: async (request: WriteTextFileRequest): Promise<WriteTextFileResponse> => {
          if (this.handlers.onWriteTextFile) {
            return this.handlers.onWriteTextFile(request);
          }
          throw new Error('File writing not supported');
        }
      };
    };

    // Create the connection using the SDK
    this.connection = new ClientSideConnection(createClient, this.httpStream.asStream());

    // Wait for agent to be set by the connection
    if (!this.agent) {
      throw new Error('Failed to establish connection - agent not initialized');
    }

    // Initialize the session
    const initResponse = await this.agent.initialize({
      protocolVersion: PROTOCOL_VERSION,
      clientCapabilities: {},
      clientInfo: {
        name: 'goose-tui',
        version: '1.0.0'
      }
    });

    // Get session ID from the HTTP stream (extracted from response header)
    const sessionId = this.httpStream.getSessionId();
    if (!sessionId) {
      throw new Error('No session ID received from server');
    }
    this.sessionId = sessionId;

    return sessionId;
  }

  /**
   * Get the Agent interface for making requests
   */
  getAgent(): Agent {
    if (!this.agent) {
      throw new Error('Not connected');
    }
    return this.agent;
  }

  /**
   * Get the current session ID
   */
  getSessionId(): string | null {
    return this.sessionId;
  }

  /**
   * Send a prompt to the agent
   */
  async prompt(message: string): Promise<void> {
    if (!this.agent || !this.sessionId) {
      throw new Error('Not connected');
    }

    await this.agent.prompt({
      sessionId: this.sessionId,
      messages: [{
        role: 'user',
        content: {
          type: 'text',
          text: message
        }
      }]
    });
  }

  /**
   * Cancel the current operation
   */
  async cancel(): Promise<void> {
    if (!this.agent || !this.sessionId) {
      throw new Error('Not connected');
    }

    await this.agent.cancel({
      sessionId: this.sessionId
    });
  }

  /**
   * Disconnect from the server
   */
  disconnect(): void {
    if (this.httpStream) {
      this.httpStream.close();
      this.httpStream = null;
    }
    this.connection = null;
    this.agent = null;
    this.sessionId = null;
  }

  /**
   * Wait for the connection to close
   */
  async waitForClose(): Promise<void> {
    if (this.connection) {
      await this.connection.closed;
    }
  }
}

/**
 * HTTP/SSE Stream implementation for the ACP SDK
 * 
 * This creates a Stream interface that:
 * - Sends messages via HTTP POST
 * - Receives messages via Server-Sent Events (SSE)
 */
class HttpSseStream {
  private serverUrl: string;
  private headers: Record<string, string>;
  private sessionId: string | null = null;
  private closed = false;
  
  // For readable stream
  private readController: ReadableStreamDefaultController<AnyMessage> | null = null;
  private pendingMessages: AnyMessage[] = [];
  
  readonly readable: ReadableStream<AnyMessage>;
  readonly writable: WritableStream<AnyMessage>;

  constructor(options: { serverUrl: string; headers?: Record<string, string> }) {
    this.serverUrl = options.serverUrl.replace(/\/$/, '');
    this.headers = options.headers || {};

    // Create readable stream
    this.readable = new ReadableStream<AnyMessage>({
      start: (controller) => {
        this.readController = controller;
        // Flush any pending messages
        for (const msg of this.pendingMessages) {
          controller.enqueue(msg);
        }
        this.pendingMessages = [];
      },
      cancel: () => {
        this.close();
      }
    });

    // Create writable stream
    this.writable = new WritableStream<AnyMessage>({
      write: async (message) => {
        await this.sendMessage(message);
      },
      close: () => {
        this.close();
      }
    });
  }

  /**
   * Get the session ID (extracted from response header after initialize)
   */
  getSessionId(): string | null {
    return this.sessionId;
  }

  /**
   * Set session ID manually
   */
  setSessionId(sessionId: string): void {
    this.sessionId = sessionId;
  }

  private enqueueMessage(message: AnyMessage): void {
    if (this.readController) {
      this.readController.enqueue(message);
    } else {
      this.pendingMessages.push(message);
    }
  }

  private async sendMessage(message: AnyMessage): Promise<void> {
    const isInitialize = 'method' in message && message.method === 'initialize';
    
    if (!isInitialize && !this.sessionId) {
      throw new Error('No session ID for non-initialize message');
    }

    const url = `${this.serverUrl}/acp`;
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      'Accept': 'application/json, text/event-stream',
      ...this.headers
    };
    
    if (this.sessionId) {
      headers['Acp-Session-Id'] = this.sessionId;
    }

    const response = await fetch(url, {
      method: 'POST',
      headers,
      body: JSON.stringify(message)
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`HTTP ${response.status}: ${errorText}`);
    }

    // For initialize, extract session ID from response header
    if (isInitialize) {
      const sessionIdHeader = response.headers.get('Acp-Session-Id');
      if (sessionIdHeader) {
        this.sessionId = sessionIdHeader;
      }
    }

    // Response is SSE stream - read events from it
    if (response.headers.get('content-type')?.includes('text/event-stream')) {
      this.readSseResponse(response);
    } else {
      // Parse JSON response and enqueue it
      const text = await response.text();
      if (text) {
        try {
          const responseMessage = JSON.parse(text) as AnyMessage;
          this.enqueueMessage(responseMessage);
        } catch {
          // Not JSON, that's okay for some responses
        }
      }
    }
  }

  private async readSseResponse(response: Response): Promise<void> {
    const reader = response.body?.getReader();
    if (!reader) return;

    const decoder = new TextDecoder();
    let buffer = '';

    try {
      while (!this.closed) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split('\n');
        buffer = lines.pop() || '';

        for (const line of lines) {
          if (line.startsWith('data:')) {
            const data = line.slice(5).trim();
            if (data) {
              try {
                const message = JSON.parse(data) as AnyMessage;
                this.enqueueMessage(message);
              } catch (err) {
                console.error('Failed to parse SSE message:', data, err);
              }
            }
          }
        }
      }
    } catch (err) {
      if (!this.closed) {
        console.error('SSE read error:', err);
      }
    } finally {
      reader.releaseLock();
    }
  }

  close(): void {
    if (this.closed) return;
    this.closed = true;

    if (this.readController) {
      try {
        this.readController.close();
      } catch {
        // Already closed
      }
    }
  }

  asStream(): Stream {
    return {
      readable: this.readable,
      writable: this.writable
    };
  }
}
