/**
 * Stream state management: types, interfaces, reducer, and initial state.
 *
 * Extracted from useChatStream to isolate state transition logic,
 * making it independently testable.
 */

import type { Message, Session, TokenState } from '../../api';
import { ChatState } from '../../types/chatState';
import type { NotificationEvent } from '../../types/message';

// ── State ────────────────────────────────────────────────────────────

export interface StreamState {
  messages: Message[];
  session: Session | undefined;
  chatState: ChatState;
  sessionLoadError: string | undefined;
  tokenState: TokenState;
  notifications: NotificationEvent[];
}

export const initialTokenState: TokenState = {
  inputTokens: 0,
  outputTokens: 0,
  totalTokens: 0,
  accumulatedInputTokens: 0,
  accumulatedOutputTokens: 0,
  accumulatedTotalTokens: 0,
};

export const initialState: StreamState = {
  messages: [],
  session: undefined,
  chatState: ChatState.Idle,
  sessionLoadError: undefined,
  tokenState: initialTokenState,
  notifications: [],
};

// ── Actions ──────────────────────────────────────────────────────────

export type StreamAction =
  | { type: 'SET_MESSAGES'; payload: Message[] }
  | { type: 'SET_SESSION'; payload: Session | undefined }
  | { type: 'SET_CHAT_STATE'; payload: ChatState }
  | { type: 'SET_SESSION_LOAD_ERROR'; payload: string | undefined }
  | { type: 'SET_TOKEN_STATE'; payload: TokenState }
  | { type: 'ADD_NOTIFICATION'; payload: NotificationEvent }
  | { type: 'CLEAR_NOTIFICATIONS' }
  | {
      type: 'SESSION_LOADED';
      payload: {
        session: Session;
        messages: Message[];
        tokenState: TokenState;
      };
    }
  | { type: 'RESET_FOR_NEW_SESSION' }
  | { type: 'START_STREAMING' }
  | { type: 'STREAM_ERROR'; payload: string }
  | { type: 'STREAM_FINISH'; payload?: string };

// ── Reducer ──────────────────────────────────────────────────────────

export function streamReducer(state: StreamState, action: StreamAction): StreamState {
  switch (action.type) {
    case 'SET_MESSAGES':
      return { ...state, messages: action.payload };

    case 'SET_SESSION':
      return { ...state, session: action.payload };

    case 'SET_CHAT_STATE':
      return { ...state, chatState: action.payload };

    case 'SET_SESSION_LOAD_ERROR':
      return { ...state, sessionLoadError: action.payload };

    case 'SET_TOKEN_STATE':
      return { ...state, tokenState: action.payload };

    case 'ADD_NOTIFICATION':
      return { ...state, notifications: [...state.notifications, action.payload] };

    case 'CLEAR_NOTIFICATIONS':
      return { ...state, notifications: [] };

    case 'SESSION_LOADED':
      return {
        ...state,
        session: action.payload.session,
        messages: action.payload.messages,
        tokenState: action.payload.tokenState,
        chatState: ChatState.Idle,
        sessionLoadError: undefined,
      };

    case 'RESET_FOR_NEW_SESSION':
      return {
        ...state,
        messages: [],
        session: undefined,
        sessionLoadError: undefined,
        chatState: ChatState.LoadingConversation,
      };

    case 'START_STREAMING':
      return {
        ...state,
        chatState: ChatState.Streaming,
        notifications: [],
      };

    case 'STREAM_ERROR':
      return {
        ...state,
        sessionLoadError: action.payload,
        chatState: ChatState.Idle,
      };

    case 'STREAM_FINISH':
      return {
        ...state,
        sessionLoadError: action.payload,
        chatState: ChatState.Idle,
      };

    default:
      return state;
  }
}
