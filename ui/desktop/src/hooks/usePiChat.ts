/**
 * React hook for Pi agent integration.
 *
 * This hook provides the same interface as useChatStream but uses Pi
 * running in the Electron main process instead of goosed.
 *
 * Architecture:
 * - Renderer calls window.electron.pi.* IPC methods
 * - Main process manages Pi session and streams events back
 * - Events are translated to Goose-compatible Message format
 */

import { useCallback, useEffect, useReducer, useRef } from 'react';
import { ChatState } from '../types/chatState';
import { Message, Session, TokenState } from '../api';
import { createUserMessage, NotificationEvent, UserInput } from '../types/message';
import { errorMessage } from '../utils/conversionUtils';

// Debug logging to file
const piLog = (msg: string, ...args: unknown[]) => {
  const line = `[${new Date().toISOString()}] ${msg} ${args.map(a => JSON.stringify(a)).join(' ')}\n`;
  console.log('[usePiChat]', msg, ...args);
  // Write to localStorage for debugging (renderer can't write files directly)
  const existing = localStorage.getItem('pi-debug-log') || '';
  localStorage.setItem('pi-debug-log', existing + line);
};

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
  isPiAvailable: boolean;
}

type PiChatAction =
  | { type: 'SET_MESSAGES'; payload: Message[] }
  | { type: 'SET_SESSION'; payload: Session | undefined }
  | { type: 'SET_CHAT_STATE'; payload: ChatState }
  | { type: 'SET_SESSION_LOAD_ERROR'; payload: string | undefined }
  | { type: 'SET_TOKEN_STATE'; payload: TokenState }
  | { type: 'SET_PI_AVAILABLE'; payload: boolean }
  | { type: 'ADD_MESSAGE'; payload: Message }
  | { type: 'UPDATE_LAST_MESSAGE'; payload: Message }
  | { type: 'START_STREAMING' }
  | { type: 'STREAM_FINISH'; payload?: string }
  | { type: 'SESSION_STARTED' };

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
  isPiAvailable: false,
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

    case 'SET_PI_AVAILABLE':
      return { ...state, isPiAvailable: action.payload };

    case 'ADD_MESSAGE':
      return { ...state, messages: [...state.messages, action.payload] };

    case 'UPDATE_LAST_MESSAGE': {
      const messages = [...state.messages];
      if (messages.length > 0 && messages[messages.length - 1].role === action.payload.role) {
        messages[messages.length - 1] = action.payload;
      } else {
        messages.push(action.payload);
      }
      return { ...state, messages };
    }

    case 'START_STREAMING':
      return { ...state, chatState: ChatState.Streaming };

    case 'STREAM_FINISH':
      return {
        ...state,
        chatState: ChatState.Idle,
        sessionLoadError: action.payload,
      };

    case 'SESSION_STARTED':
      return {
        ...state,
        chatState: ChatState.Idle,
        session: {
          id: `pi-${Date.now()}`,
          name: 'Pi Session',
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        } as Session,
      };

    default:
      return state;
  }
}

export function usePiChat({
  sessionId: _sessionId,
  onStreamFinish,
  onSessionLoaded,
}: UsePiChatProps): UsePiChatReturn {
  const [state, dispatch] = useReducer(piChatReducer, initialState);
  const stateRef = useRef(state);
  stateRef.current = state;
  const sessionStartedRef = useRef(false);

  // Get working directory from app config
  const workingDir = (window.appConfig?.get('GOOSE_WORKING_DIR') as string) || process.cwd();

  // Check Pi availability and auto-start session on mount
  useEffect(() => {
    if (sessionStartedRef.current) return;

    (async () => {
      try {
        console.log('[usePiChat] Checking Pi availability...');
        const available = await window.electron.pi.isAvailable();
        console.log('[usePiChat] Pi available:', available);
        dispatch({ type: 'SET_PI_AVAILABLE', payload: available });

        if (available && !sessionStartedRef.current) {
          sessionStartedRef.current = true;
          dispatch({ type: 'SET_CHAT_STATE', payload: ChatState.LoadingConversation });

          console.log('[usePiChat] Starting Pi session with workingDir:', workingDir);
          const result = await window.electron.pi.startSession({ workingDir });
          console.log('[usePiChat] Session start result:', result);
          if (result.success) {
            dispatch({ type: 'SESSION_STARTED' });
            onSessionLoaded?.();
          } else {
            dispatch({ type: 'SET_SESSION_LOAD_ERROR', payload: 'Failed to start Pi session' });
          }
        }
      } catch (err) {
        console.error('[usePiChat] Error:', err);
        dispatch({ type: 'SET_PI_AVAILABLE', payload: false });
        dispatch({ type: 'SET_SESSION_LOAD_ERROR', payload: errorMessage(err) });
      }
    })();
  }, [workingDir, onSessionLoaded]);

  // Listen for Pi events from main process
  useEffect(() => {
    const handlePiEvent = (_event: { type: string }) => {
      // Raw Pi events are for debugging - we use translated pi:message events
    };

    const handlePiMessage = (gooseMessage: Message) => {
      // Handle translated Goose messages (these have proper metadata)
      if (gooseMessage && gooseMessage.metadata) {
        dispatch({ type: 'UPDATE_LAST_MESSAGE', payload: gooseMessage });
      }
    };

    const handlePiComplete = () => {
      dispatch({ type: 'STREAM_FINISH' });
      onStreamFinish();
    };

    const handlePiError = (data: { error: string }) => {
      dispatch({ type: 'STREAM_FINISH', payload: data.error });
      onStreamFinish();
    };

    // Register listeners via IPC
    window.electron.on('pi:event', (_e, data) => handlePiEvent(data as { type: string; message?: Message }));
    window.electron.on('pi:message', (_e, data) => handlePiMessage(data as Message));
    window.electron.on('pi:complete', handlePiComplete);
    window.electron.on('pi:error', (_e, data) => handlePiError(data as { error: string }));

    return () => {
      window.electron.off('pi:event', handlePiEvent as () => void);
      window.electron.off('pi:message', handlePiMessage as () => void);
      window.electron.off('pi:complete', handlePiComplete);
      window.electron.off('pi:error', handlePiError as () => void);
    };
  }, [onStreamFinish]);

  const handleSubmit = useCallback(
    async (input: UserInput) => {
      console.log('[usePiChat] handleSubmit called with:', input.msg);
      const userMessage = createUserMessage(input.msg, input.images);
      dispatch({ type: 'ADD_MESSAGE', payload: userMessage });
      dispatch({ type: 'START_STREAMING' });

      try {
        console.log('[usePiChat] Calling pi.prompt...');
        const result = await window.electron.pi.prompt(input.msg);
        console.log('[usePiChat] prompt result:', result);

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
      // Pi doesn't support elicitation yet - no-op
      console.warn('[usePiChat] Elicitation not supported by Pi agent');
    },
    []
  );

  const setRecipeUserParams = useCallback(async (_values: Record<string, string>) => {
    // Pi doesn't support recipe params yet - no-op
    console.warn('[usePiChat] Recipe params not supported by Pi agent');
  }, []);

  const stopStreaming = useCallback(async () => {
    try {
      await window.electron.pi.stopSession();
    } catch {
      // Ignore abort errors
    }
    dispatch({ type: 'SET_CHAT_STATE', payload: ChatState.Idle });
  }, []);

  const setChatState = useCallback((newState: ChatState) => {
    dispatch({ type: 'SET_CHAT_STATE', payload: newState });
  }, []);

  const onMessageUpdate = useCallback(
    async (_messageId: string, _newContent: string, _editType?: 'fork' | 'edit') => {
      // Pi doesn't support message editing yet - no-op
      console.warn('[usePiChat] Message editing not supported by Pi agent');
    },
    []
  );

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
    notifications: new Map(),
    onMessageUpdate,
  };
}
