/**
 * React hook for Pi agent chat.
 *
 * This is the primary chat hook when using Pi as the backend.
 * It provides a similar interface to useChatStream for UI compatibility.
 */

import { useCallback, useEffect, useMemo, useReducer, useRef } from 'react';
import { ChatState } from '../types/chatState';
import { Message, Session, TokenState } from '../api';
import { createUserMessage, NotificationEvent, UserInput } from '../types/message';
import { errorMessage } from '../utils/conversionUtils';
import { AppEvents } from '../constants/events';

// Re-export Session type compatible with Goose
export type PiSession = Session;

interface UsePiChatProps {
  sessionId: string;
  onStreamFinish: () => void;
  onSessionLoaded?: () => void;
}

interface UsePiChatReturn {
  session?: Session;
  messages: Message[];
  chatState: ChatState;
  setChatState: (state: ChatState) => void;
  handleSubmit: (input: UserInput) => Promise<void>;
  submitElicitationResponse: (
    elicitationId: string,
    userData: Record<string, unknown>
  ) => Promise<void>;
  setRecipeUserParams: (values: Record<string, string>) => Promise<void>;
  stopStreaming: () => void;
  sessionLoadError?: string;
  tokenState: TokenState;
  notifications: Map<string, NotificationEvent[]>;
  onMessageUpdate: (
    messageId: string,
    newContent: string,
    editType?: 'fork' | 'edit'
  ) => Promise<void>;
}

interface PiChatState {
  messages: Message[];
  session: Session | undefined;
  chatState: ChatState;
  sessionLoadError: string | undefined;
  tokenState: TokenState;
  notifications: NotificationEvent[];
}

type PiChatAction =
  | { type: 'SET_MESSAGES'; payload: Message[] }
  | { type: 'SET_SESSION'; payload: Session | undefined }
  | { type: 'SET_CHAT_STATE'; payload: ChatState }
  | { type: 'SET_SESSION_LOAD_ERROR'; payload: string | undefined }
  | { type: 'SET_TOKEN_STATE'; payload: TokenState }
  | { type: 'ADD_MESSAGE'; payload: Message }
  | { type: 'UPDATE_MESSAGE'; payload: Message }
  | { type: 'ADD_NOTIFICATION'; payload: NotificationEvent }
  | { type: 'START_STREAMING' }
  | { type: 'STREAM_FINISH'; payload?: string }
  | {
      type: 'SESSION_LOADED';
      payload: {
        session: Session;
        messages: Message[];
        tokenState: TokenState;
      };
    }
  | { type: 'RESET_FOR_NEW_SESSION' };

const initialTokenState: TokenState = {
  inputTokens: 0,
  outputTokens: 0,
  totalTokens: 0,
  accumulatedInputTokens: 0,
  accumulatedOutputTokens: 0,
  accumulatedTotalTokens: 0,
};

const initialState: PiChatState = {
  messages: [],
  session: undefined,
  chatState: ChatState.Idle,
  sessionLoadError: undefined,
  tokenState: initialTokenState,
  notifications: [],
};

function piChatReducer(state: PiChatState, action: PiChatAction): PiChatState {
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

    case 'ADD_MESSAGE':
      return { ...state, messages: [...state.messages, action.payload] };

    case 'UPDATE_MESSAGE': {
      const messages = [...state.messages];
      const idx = messages.findIndex((m) => m.id === action.payload.id);
      if (idx >= 0) {
        messages[idx] = action.payload;
      } else {
        // New message, add it
        messages.push(action.payload);
      }
      return { ...state, messages };
    }

    case 'ADD_NOTIFICATION':
      return { ...state, notifications: [...state.notifications, action.payload] };

    case 'START_STREAMING':
      return { ...state, chatState: ChatState.Streaming, notifications: [] };

    case 'STREAM_FINISH':
      return {
        ...state,
        chatState: ChatState.Idle,
        sessionLoadError: action.payload,
      };

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

    default:
      return state;
  }
}

// Convert Pi session to Goose Session type
function toGooseSession(piSession: {
  id: string;
  name: string;
  created_at: string;
  updated_at: string;
  working_dir: string;
  message_count: number;
  conversation: Message[];
  input_tokens: number;
  output_tokens: number;
  total_tokens: number;
  accumulated_input_tokens: number;
  accumulated_output_tokens: number;
  accumulated_total_tokens: number;
}): Session {
  return {
    id: piSession.id,
    name: piSession.name,
    created_at: piSession.created_at,
    updated_at: piSession.updated_at,
    working_dir: piSession.working_dir,
    message_count: piSession.message_count,
    conversation: piSession.conversation,
    input_tokens: piSession.input_tokens,
    output_tokens: piSession.output_tokens,
    total_tokens: piSession.total_tokens,
    accumulated_input_tokens: piSession.accumulated_input_tokens,
    accumulated_output_tokens: piSession.accumulated_output_tokens,
    accumulated_total_tokens: piSession.accumulated_total_tokens,
    // These fields aren't used by Pi but the type expects them
    extension_data: {},
  };
}

export function usePiChat({
  sessionId,
  onStreamFinish,
  onSessionLoaded,
}: UsePiChatProps): UsePiChatReturn {
  const [state, dispatch] = useReducer(piChatReducer, initialState);
  const stateRef = useRef(state);
  stateRef.current = state;
  const lastInteractionTimeRef = useRef<number>(Date.now());

  // Load or create session on mount
  useEffect(() => {
    if (!sessionId) return;

    dispatch({ type: 'RESET_FOR_NEW_SESSION' });

    let cancelled = false;

    (async () => {
      try {
        // Check if Pi is available
        const available = await window.electron.pi.isAvailable();
        if (!available) {
          dispatch({ type: 'SET_SESSION_LOAD_ERROR', payload: 'Pi agent is not available' });
          return;
        }

        // Try to resume existing session first
        const resumeResult = await window.electron.pi.resumeSession(sessionId);
        
        if (cancelled) return;

        if (resumeResult.success && resumeResult.session) {
          const session = toGooseSession(resumeResult.session as Parameters<typeof toGooseSession>[0]);
          dispatch({
            type: 'SESSION_LOADED',
            payload: {
              session,
              messages: resumeResult.session.conversation as Message[],
              tokenState: {
                inputTokens: resumeResult.session.input_tokens,
                outputTokens: resumeResult.session.output_tokens,
                totalTokens: resumeResult.session.total_tokens,
                accumulatedInputTokens: resumeResult.session.accumulated_input_tokens,
                accumulatedOutputTokens: resumeResult.session.accumulated_output_tokens,
                accumulatedTotalTokens: resumeResult.session.accumulated_total_tokens,
              },
            },
          });
          onSessionLoaded?.();
        } else {
          // Session doesn't exist - create a new one with this ID
          // Get working directory from config
          const workingDir =
            (window.appConfig?.get('GOOSE_WORKING_DIR') as string) || process.cwd?.() || '/';

          const createResult = await window.electron.pi.createSession({ workingDir });

          if (cancelled) return;

          if (createResult.success && createResult.session) {
            const session = toGooseSession(createResult.session as Parameters<typeof toGooseSession>[0]);
            dispatch({
              type: 'SESSION_LOADED',
              payload: {
                session,
                messages: [],
                tokenState: initialTokenState,
              },
            });
            onSessionLoaded?.();
          } else {
            dispatch({
              type: 'SET_SESSION_LOAD_ERROR',
              payload: createResult.error || 'Failed to create Pi session',
            });
          }
        }
      } catch (error) {
        if (cancelled) return;
        console.error('[usePiChat] Error loading session:', error);
        dispatch({ type: 'SET_SESSION_LOAD_ERROR', payload: errorMessage(error) });
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [sessionId, onSessionLoaded]);

  // Listen for Pi events from main process
  useEffect(() => {
    const handlePiMessage = (_e: unknown, gooseMessage: Message) => {
      if (gooseMessage && gooseMessage.metadata) {
        dispatch({ type: 'UPDATE_MESSAGE', payload: gooseMessage });
      }
    };

    const handlePiNotification = (_e: unknown, notification: NotificationEvent) => {
      if (notification && notification.type === 'Notification') {
        dispatch({ type: 'ADD_NOTIFICATION', payload: notification });
      }
    };

    const handlePiComplete = (_e: unknown, data?: { messages?: Message[] }) => {
      // Update with final messages if provided
      if (data?.messages) {
        dispatch({ type: 'SET_MESSAGES', payload: data.messages });
      }
      dispatch({ type: 'STREAM_FINISH' });
      onStreamFinish();
    };

    const handlePiError = (_e: unknown, data: { error: string }) => {
      dispatch({ type: 'STREAM_FINISH', payload: data.error });
      onStreamFinish();
    };

    // Register listeners via IPC
    window.electron.on('pi:message', handlePiMessage as () => void);
    window.electron.on('pi:notification', handlePiNotification as () => void);
    window.electron.on('pi:complete', handlePiComplete as () => void);
    window.electron.on('pi:error', handlePiError as () => void);

    return () => {
      window.electron.off('pi:message', handlePiMessage as () => void);
      window.electron.off('pi:notification', handlePiNotification as () => void);
      window.electron.off('pi:complete', handlePiComplete as () => void);
      window.electron.off('pi:error', handlePiError as () => void);
    };
  }, [onStreamFinish]);

  const handleSubmit = useCallback(
    async (input: UserInput) => {
      const currentState = stateRef.current;

      // Guard: Don't submit if session hasn't been loaded yet
      if (!currentState.session || currentState.chatState === ChatState.LoadingConversation) {
        return;
      }

      const hasExistingMessages = currentState.messages.length > 0;
      const hasNewMessage = input.msg.trim().length > 0 || input.images.length > 0;

      // Don't submit if there's no message
      if (!hasNewMessage) {
        return;
      }

      lastInteractionTimeRef.current = Date.now();

      // Emit session-created event for first message in a new session
      if (!hasExistingMessages && hasNewMessage) {
        window.dispatchEvent(new CustomEvent(AppEvents.SESSION_CREATED));
      }

      // Add user message to UI immediately
      const userMessage = createUserMessage(input.msg, input.images);
      dispatch({ type: 'ADD_MESSAGE', payload: userMessage });
      dispatch({ type: 'START_STREAMING' });

      try {
        const result = await window.electron.pi.prompt(input.msg);

        if (!result.success && result.error) {
          dispatch({ type: 'STREAM_FINISH', payload: result.error });
          onStreamFinish();
        }
        // Success case is handled by IPC event listeners
      } catch (error) {
        console.error('[usePiChat] prompt error:', error);
        dispatch({ type: 'STREAM_FINISH', payload: errorMessage(error) });
        onStreamFinish();
      }
    },
    [onStreamFinish]
  );

  const submitElicitationResponse = useCallback(
    async (_elicitationId: string, _userData: Record<string, unknown>) => {
      // Pi doesn't support elicitation - this is a no-op
      console.warn('[usePiChat] Elicitation not supported by Pi agent');
    },
    []
  );

  const setRecipeUserParams = useCallback(async (_values: Record<string, string>) => {
    // Pi doesn't support recipe params - this is a no-op
    console.warn('[usePiChat] Recipe params not supported by Pi agent');
  }, []);

  const stopStreaming = useCallback(async () => {
    try {
      await window.electron.pi.abort();
    } catch {
      // Ignore abort errors
    }
    dispatch({ type: 'SET_CHAT_STATE', payload: ChatState.Idle });
    lastInteractionTimeRef.current = Date.now();
  }, []);

  const setChatState = useCallback((newState: ChatState) => {
    dispatch({ type: 'SET_CHAT_STATE', payload: newState });
  }, []);

  const onMessageUpdate = useCallback(
    async (_messageId: string, _newContent: string, _editType?: 'fork' | 'edit') => {
      // Pi doesn't support message editing - this is a no-op
      console.warn('[usePiChat] Message editing not supported by Pi agent');
    },
    []
  );

  // Build notifications map grouped by request_id (for tool call progress)
  const notificationsMap = useMemo(() => {
    const map = new Map<string, NotificationEvent[]>();
    for (const notification of state.notifications) {
      const requestId = notification.request_id;
      if (!map.has(requestId)) {
        map.set(requestId, []);
      }
      map.get(requestId)!.push(notification);
    }
    return map;
  }, [state.notifications]);

  return {
    session: state.session,
    messages: state.messages,
    chatState: state.chatState,
    setChatState,
    handleSubmit,
    submitElicitationResponse,
    setRecipeUserParams,
    stopStreaming,
    sessionLoadError: state.sessionLoadError,
    tokenState: state.tokenState,
    notifications: notificationsMap,
    onMessageUpdate,
  };
}
