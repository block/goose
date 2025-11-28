import { Message, MessageEvent, reply, resumeAgent } from '../api';
import { createClient, createConfig, type Client } from '../api/client';
import { NotificationEvent } from '../types/message';
import { SessionState, WorkerConfig } from './types';

/**
 * Manages multiple chat sessions in a Web Worker
 * Handles streaming, state management, and concurrent sessions
 */
export class SessionManager {
  private sessions: Map<string, SessionState> = new Map();
  private abortControllers: Map<string, AbortController> = new Map();
  private client: Client;

  constructor(config: WorkerConfig) {
    // Create API client with worker config
    this.client = createClient(
      createConfig({
        baseUrl: config.apiBaseUrl,
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': config.secretKey,
        },
      })
    );

    console.log('[SessionManager] Initialized with baseUrl:', config.apiBaseUrl);
  }

  /**
   * Initialize a new session
   */
  initSession(sessionId: string): SessionState {
    if (this.sessions.has(sessionId)) {
      return this.sessions.get(sessionId)!;
    }

    const state: SessionState = {
      sessionId,
      messages: [],
      tokenState: {
        inputTokens: 0,
        outputTokens: 0,
        totalTokens: 0,
        accumulatedInputTokens: 0,
        accumulatedOutputTokens: 0,
        accumulatedTotalTokens: 0,
      },
      notifications: [],
      streamState: 'idle',
      lastUpdated: Date.now(),
    };

    this.sessions.set(sessionId, state);
    return state;
  }

  /**
   * Load an existing session from the server
   */
  async loadSession(sessionId: string): Promise<SessionState> {
    let state = this.sessions.get(sessionId);
    if (!state) {
      state = this.initSession(sessionId);
    }

    state.streamState = 'loading';
    this.sessions.set(sessionId, state);

    try {
      const response = await resumeAgent({
        client: this.client,
        body: {
          session_id: sessionId,
          load_model_and_extensions: true,
        },
        throwOnError: true,
      });

      const session = response.data;
      state.session = session;
      state.messages = session?.conversation || [];
      state.streamState = 'idle';
      state.lastUpdated = Date.now();

      this.sessions.set(sessionId, state);
      return state;
    } catch (error) {
      state.streamState = 'error';
      state.error = error instanceof Error ? error.message : 'Failed to load session';
      this.sessions.set(sessionId, state);
      throw error;
    }
  }

  /**
   * Start streaming for a session
   */
  async startStream(
    sessionId: string,
    _userMessage: string,
    messages: Message[],
    onUpdate: (state: Partial<SessionState>) => void
  ): Promise<void> {
    const state = this.sessions.get(sessionId);
    if (!state) {
      throw new Error(`Session ${sessionId} not found`);
    }

    if (!state.session) {
      throw new Error(`Session ${sessionId} not loaded`);
    }

    // Stop any existing stream
    this.stopStream(sessionId);

    // Create new abort controller
    const abortController = new AbortController();
    this.abortControllers.set(sessionId, abortController);

    // IMPORTANT: Immediately update state with the user message
    // This ensures the UI gets the message from the worker (backend as source of truth)
    state.messages = messages;
    state.streamState = 'streaming';
    state.error = undefined;
    state.lastUpdated = Date.now();
    this.sessions.set(sessionId, state);

    // Notify subscribers immediately with the user message
    onUpdate({
      streamState: 'streaming',
      messages: messages,
    });

    try {
      const { stream } = await reply({
        client: this.client,
        body: {
          session_id: sessionId,
          messages,
        },
        throwOnError: true,
        signal: abortController.signal,
      });

      if (!stream) {
        throw new Error('No stream in response');
      }

      let currentMessages = [...messages];

      for await (const event of stream) {
        // Check if stream was aborted
        if (abortController.signal.aborted) {
          break;
        }

        await this.handleStreamEvent(sessionId, event, currentMessages, onUpdate);

        // Update currentMessages if we got new ones
        const updatedState = this.sessions.get(sessionId);
        if (updatedState) {
          currentMessages = updatedState.messages;
        }
      }

      // Stream finished successfully
      state.streamState = 'idle';
      state.lastUpdated = Date.now();
      this.sessions.set(sessionId, state);

      this.abortControllers.delete(sessionId);
      onUpdate({ streamState: 'idle' });
    } catch (error) {
      if (error instanceof Error && error.name === 'AbortError') {
        // Stream was intentionally stopped
        state.streamState = 'idle';
        this.sessions.set(sessionId, state);
        onUpdate({ streamState: 'idle' });
      } else {
        // Actual error
        const errorMessage = error instanceof Error ? error.message : 'Stream error';
        state.streamState = 'error';
        state.error = errorMessage;
        state.lastUpdated = Date.now();
        this.sessions.set(sessionId, state);

        this.abortControllers.delete(sessionId);
        onUpdate({ streamState: 'error', error: errorMessage });
        throw error;
      }
    }
  }

  /**
   * Handle individual stream events
   */
  private async handleStreamEvent(
    sessionId: string,
    event: MessageEvent,
    currentMessages: Message[],
    onUpdate: (state: Partial<SessionState>) => void
  ): Promise<void> {
    const state = this.sessions.get(sessionId);
    if (!state) return;

    switch (event.type) {
      case 'Message': {
        const msg = event.message;
        const updatedMessages = this.pushMessage(currentMessages, msg);

        state.messages = updatedMessages;
        state.lastUpdated = Date.now();

        if (event.token_state) {
          state.tokenState = event.token_state;
        }

        this.sessions.set(sessionId, state);
        onUpdate({
          messages: updatedMessages,
          tokenState: event.token_state,
        });
        break;
      }

      case 'Error': {
        console.error('Stream event error:', event.error);
        throw new Error('Stream error: ' + event.error);
      }

      case 'Finish': {
        if (event.token_state) {
          state.tokenState = event.token_state;
          this.sessions.set(sessionId, state);
          onUpdate({ tokenState: event.token_state });
        }
        break;
      }

      case 'ModelChange': {
        console.log('Model changed:', event.model, event.mode);
        break;
      }

      case 'UpdateConversation': {
        state.messages = event.conversation;
        state.lastUpdated = Date.now();
        this.sessions.set(sessionId, state);
        onUpdate({ messages: event.conversation });
        break;
      }

      case 'Notification': {
        const notification: NotificationEvent = {
          type: 'Notification',
          request_id: event.request_id,
          message: event.message,
        };
        state.notifications.push(notification);
        state.lastUpdated = Date.now();
        this.sessions.set(sessionId, state);
        onUpdate({ notifications: [...state.notifications] });
        break;
      }

      case 'Ping': {
        // Keep-alive ping, no action needed
        break;
      }

      default: {
        console.warn('Unhandled event type:', (event as MessageEvent)['type']);
        break;
      }
    }
  }

  /**
   * Push a message into the message array, merging text content if it's the same message
   */
  private pushMessage(currentMessages: Message[], incomingMsg: Message): Message[] {
    const lastMsg = currentMessages[currentMessages.length - 1];

    if (lastMsg?.id && lastMsg.id === incomingMsg.id) {
      const lastContent = lastMsg.content[lastMsg.content.length - 1];
      const newContent = incomingMsg.content[incomingMsg.content.length - 1];

      if (
        lastContent?.type === 'text' &&
        newContent?.type === 'text' &&
        incomingMsg.content.length === 1
      ) {
        lastContent.text += newContent.text;
      } else {
        lastMsg.content.push(...incomingMsg.content);
      }
      return [...currentMessages];
    } else {
      return [...currentMessages, incomingMsg];
    }
  }

  /**
   * Stop streaming for a session
   */
  stopStream(sessionId: string): void {
    const abortController = this.abortControllers.get(sessionId);
    if (abortController) {
      abortController.abort();
      this.abortControllers.delete(sessionId);
    }

    const state = this.sessions.get(sessionId);
    if (state && state.streamState === 'streaming') {
      state.streamState = 'idle';
      state.lastUpdated = Date.now();
      this.sessions.set(sessionId, state);
    }
  }

  /**
   * Destroy a session and clean up resources
   */
  destroySession(sessionId: string): void {
    this.stopStream(sessionId);
    this.sessions.delete(sessionId);
  }

  /**
   * Get current state of a session
   */
  getSessionState(sessionId: string): SessionState | null {
    return this.sessions.get(sessionId) || null;
  }

  /**
   * Get all active sessions
   */
  getAllSessions(): SessionState[] {
    return Array.from(this.sessions.values());
  }

  /**
   * Get count of currently streaming sessions
   */
  getStreamingCount(): number {
    return Array.from(this.sessions.values()).filter((s) => s.streamState === 'streaming').length;
  }

  /**
   * Clean up old sessions (LRU eviction)
   */
  cleanupOldSessions(maxSessions: number = 10): void {
    if (this.sessions.size <= maxSessions) return;

    // Sort by last updated time
    const sorted = Array.from(this.sessions.entries()).sort(
      ([, a], [, b]) => a.lastUpdated - b.lastUpdated
    );

    // Remove oldest sessions that aren't streaming
    const toRemove = sorted
      .filter(([, state]) => state.streamState !== 'streaming')
      .slice(0, this.sessions.size - maxSessions);

    for (const [sessionId] of toRemove) {
      this.destroySession(sessionId);
    }
  }
}
