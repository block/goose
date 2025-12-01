import { Message, TokenState, Session } from '../api';
import { NotificationEvent } from '../types/message';

/**
 * State of a session stream
 */
export type StreamState = 'idle' | 'loading' | 'streaming' | 'paused' | 'error';

/**
 * Complete state for a single session managed by the worker
 */
export interface SessionState {
  sessionId: string;
  session?: Session;
  messages: Message[];
  tokenState: TokenState;
  notifications: NotificationEvent[];
  streamState: StreamState;
  error?: string;
  lastUpdated: number;
}

/**
 * Messages sent from UI to Worker
 */
export type WorkerCommand =
  | {
      type: 'INIT';
      config: WorkerConfig;
    }
  | {
      type: 'INIT_SESSION';
      sessionId: string;
    }
  | {
      type: 'LOAD_SESSION';
      sessionId: string;
    }
  | {
      type: 'START_STREAM';
      sessionId: string;
      userMessage: string;
      messages: Message[];
    }
  | {
      type: 'STOP_STREAM';
      sessionId: string;
    }
  | {
      type: 'DESTROY_SESSION';
      sessionId: string;
    }
  | {
      type: 'GET_SESSION_STATE';
      sessionId: string;
    }
  | {
      type: 'UPDATE_RECIPE_PARAMS';
      sessionId: string;
      params: Record<string, string>;
    }
  | {
      type: 'UPDATE_SESSION';
      sessionId: string;
      session: Session;
    }
  | {
      type: 'GET_ALL_SESSIONS';
    };

/**
 * Messages sent from Worker to UI
 */
export type WorkerResponse =
  | {
      type: 'READY';
    }
  | {
      type: 'SESSION_INITIALIZED';
      sessionId: string;
    }
  | {
      type: 'SESSION_LOADED';
      sessionId: string;
      state: SessionState;
    }
  | {
      type: 'SESSION_UPDATE';
      sessionId: string;
      state: Partial<SessionState>;
    }
  | {
      type: 'MESSAGE_ADDED';
      sessionId: string;
      message: Message;
    }
  | {
      type: 'STREAM_STARTED';
      sessionId: string;
    }
  | {
      type: 'STREAM_FINISHED';
      sessionId: string;
      error?: string;
    }
  | {
      type: 'TOKEN_UPDATE';
      sessionId: string;
      tokenState: TokenState;
    }
  | {
      type: 'NOTIFICATION';
      sessionId: string;
      notification: NotificationEvent;
    }
  | {
      type: 'ERROR';
      sessionId: string;
      error: string;
    }
  | {
      type: 'SESSION_STATE';
      sessionId: string;
      state: SessionState | null;
    }
  | {
      type: 'ALL_SESSIONS';
      sessions: SessionState[];
    };

/**
 * Configuration for the session worker
 */
export interface WorkerConfig {
  apiBaseUrl: string;
  secretKey: string;
  maxConcurrentSessions?: number;
  maxMessagesPerSession?: number;
}
