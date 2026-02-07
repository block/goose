import type { AcpMessage } from '../client.js';

export type MessageHandler = (message: AcpMessage) => void;
export type ErrorHandler = (error: Error) => void;

// ACP header constant
export const HEADER_SESSION_ID = 'Acp-Session-Id';

/**
 * Transport abstraction for ACP communication.
 * Supports both HTTP/SSE and WebSocket implementations.
 */
export interface Transport {
  /**
   * Establish connection to ACP server
   * @param baseUrl - Server URL (e.g., http://localhost:3284)
   * @param sessionId - Optional session ID for reconnection
   * @returns Session ID
   */
  connect(baseUrl: string, sessionId?: string): Promise<string>;

  /**
   * Send a message to the server
   */
  send(message: AcpMessage): Promise<void>;

  /**
   * Close the connection
   */
  disconnect(): Promise<void>;

  /**
   * Register message handler
   * @returns Unsubscribe function
   */
  onMessage(handler: MessageHandler): () => void;

  /**
   * Register error handler
   * @returns Unsubscribe function
   */
  onError(handler: ErrorHandler): () => void;

  /**
   * Check if transport is connected
   */
  isConnected(): boolean;
}
