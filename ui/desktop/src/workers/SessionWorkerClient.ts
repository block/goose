import { Message, Session } from '../api';
import { SessionState, WorkerCommand, WorkerResponse, WorkerConfig } from './types';

interface PendingRequest {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  resolve: (value: any) => void;
  reject: (error: Error) => void;
}

/**
 * Client for communicating with the session worker.
 * Handles message passing, promise resolution, and subscriptions.
 */
export class SessionWorkerClient {
  private worker: Worker | null = null;
  private pendingRequests = new Map<string, PendingRequest>();
  private subscribers = new Map<string, Set<(state: Partial<SessionState>) => void>>();
  public isReady = false;
  private readyPromise: Promise<void>;
  private readyResolve!: () => void;

  constructor(config: WorkerConfig) {
    this.readyPromise = new Promise((resolve) => {
      this.readyResolve = resolve;
    });
    this.initWorker(config);
  }

  private initWorker(config: WorkerConfig) {
    this.worker = new Worker(new URL('./sessionWorker.ts', import.meta.url), {
      type: 'module',
    });

    this.worker.onmessage = this.handleWorkerMessage;
    this.worker.onerror = this.handleWorkerError;

    // Initialize the worker
    this.sendCommand({ type: 'INIT', config });
  }

  private handleWorkerMessage = (event: MessageEvent<WorkerResponse>) => {
    const message = event.data;

    // Handle initialization
    if (message.type === 'READY') {
      this.isReady = true;
      this.readyResolve();
      return;
    }

    // Handle session-specific messages
    if ('sessionId' in message) {
      const { sessionId } = message;

      switch (message.type) {
        case 'SESSION_LOADED': {
          const request = this.pendingRequests.get(`load-${sessionId}`);
          if (request) {
            request.resolve(message.state);
            this.pendingRequests.delete(`load-${sessionId}`);
          }
          break;
        }

        case 'SESSION_UPDATE': {
          // Notify all subscribers for this session
          const subscribers = this.subscribers.get(sessionId);
          if (subscribers) {
            subscribers.forEach((callback) => callback(message.state));
          }
          break;
        }

        case 'STREAM_FINISHED': {
          const request = this.pendingRequests.get(`stream-${sessionId}`);
          if (request) {
            if (message.error) {
              request.reject(new Error(message.error));
            } else {
              request.resolve(undefined);
            }
            this.pendingRequests.delete(`stream-${sessionId}`);
          }
          break;
        }

        case 'SESSION_STATE': {
          const request = this.pendingRequests.get(`state-${sessionId}`);
          if (request) {
            request.resolve(message.state);
            this.pendingRequests.delete(`state-${sessionId}`);
          }
          break;
        }

        case 'ERROR': {
          // Find any pending request for this session
          const loadRequest = this.pendingRequests.get(`load-${sessionId}`);
          const streamRequest = this.pendingRequests.get(`stream-${sessionId}`);
          const request = loadRequest || streamRequest;

          if (request) {
            request.reject(new Error(message.error));
            this.pendingRequests.delete(`load-${sessionId}`);
            this.pendingRequests.delete(`stream-${sessionId}`);
          }
          break;
        }
      }
    }
  };

  private handleWorkerError = (error: Event) => {
    console.error('Worker error:', error);
    this.isReady = false;
  };

  private sendCommand(command: WorkerCommand) {
    if (!this.worker) {
      throw new Error('Worker not initialized');
    }
    this.worker.postMessage(command);
  }

  /**
   * Wait for worker to be ready
   */
  async waitForReady(): Promise<void> {
    return this.readyPromise;
  }

  /**
   * Initialize a new session
   */
  initSession(sessionId: string) {
    this.sendCommand({ type: 'INIT_SESSION', sessionId });
  }

  /**
   * Load an existing session
   */
  async loadSession(sessionId: string): Promise<SessionState> {
    await this.waitForReady();

    return new Promise((resolve, reject) => {
      this.pendingRequests.set(`load-${sessionId}`, { resolve, reject });
      this.sendCommand({ type: 'LOAD_SESSION', sessionId });
    });
  }

  /**
   * Start streaming for a session
   */
  async startStream(sessionId: string, userMessage: string, messages: Message[]): Promise<void> {
    await this.waitForReady();

    return new Promise((resolve, reject) => {
      this.pendingRequests.set(`stream-${sessionId}`, { resolve, reject });
      this.sendCommand({ type: 'START_STREAM', sessionId, userMessage, messages });
    });
  }

  /**
   * Stop streaming for a session
   */
  stopStream(sessionId: string) {
    this.sendCommand({ type: 'STOP_STREAM', sessionId });
  }

  /**
   * Update session data in the worker (e.g., after updating recipe parameters)
   */
  async updateSession(sessionId: string, session: Session): Promise<void> {
    await this.waitForReady();

    // Send update command to worker
    this.sendCommand({
      type: 'UPDATE_SESSION',
      sessionId,
      session,
    });

    // Also notify subscribers immediately
    const subscribers = this.subscribers.get(sessionId);
    if (subscribers) {
      subscribers.forEach((callback) => {
        callback({ session });
      });
    }
  }

  /**
   * Get current state of a session
   */
  async getSessionState(sessionId: string): Promise<SessionState | null> {
    await this.waitForReady();

    return new Promise((resolve, reject) => {
      this.pendingRequests.set(`state-${sessionId}`, { resolve, reject });
      this.sendCommand({ type: 'GET_SESSION_STATE', sessionId });
    });
  }

  /**
   * Subscribe to session updates
   */
  subscribeToSession(
    sessionId: string,
    callback: (state: Partial<SessionState>) => void
  ): () => void {
    // Create subscriber set if it doesn't exist
    if (!this.subscribers.has(sessionId)) {
      this.subscribers.set(sessionId, new Set());
    }

    const subscribers = this.subscribers.get(sessionId)!;
    subscribers.add(callback);

    // Return unsubscribe function
    return () => {
      subscribers.delete(callback);
      if (subscribers.size === 0) {
        this.subscribers.delete(sessionId);
      }
    };
  }

  /**
   * Destroy a session and clean up
   */
  destroySession(sessionId: string) {
    this.sendCommand({ type: 'DESTROY_SESSION', sessionId });
    this.subscribers.delete(sessionId);

    // Clean up any pending requests
    this.pendingRequests.delete(`load-${sessionId}`);
    this.pendingRequests.delete(`stream-${sessionId}`);
    this.pendingRequests.delete(`state-${sessionId}`);
  }

  /**
   * Clean up the worker
   */
  terminate() {
    if (this.worker) {
      this.worker.terminate();
      this.worker = null;
    }
    this.isReady = false;
    this.pendingRequests.clear();
    this.subscribers.clear();
  }
}
