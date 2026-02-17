import { useCallback, useEffect, useMemo, useReducer, useRef } from 'react';
import { AppEvents } from '../constants/events';
import { ChatState } from '../types/chatState';

import {
  getSession,
  getSessionExtensions,
  Message,
  MessageEvent,
  reply,
  resumeAgent,
  Session,
  TokenState,
  updateFromSession,
  updateSessionUserRecipeValues,
  listApps,
} from '../api';

import {
  createUserMessage,
  createElicitationResponseMessage,
  getCompactingMessage,
  getThinkingMessage,
  NotificationEvent,
  UserInput,
} from '../types/message';
import { errorMessage } from '../utils/conversionUtils';
import { showExtensionLoadResults } from '../utils/extensionErrorUtils';
import { estimateExtensionLoadTimes } from '../utils/extensionLoadMetrics';
import { maybeHandlePlatformEvent } from '../utils/platform_events';
import { toastService } from '../toasts';

const resultsCache = new Map<string, { messages: Message[]; session: Session }>();

interface UseChatStreamProps {
  sessionId: string;
  onStreamFinish: () => void;
  onSessionLoaded?: () => void;
}

interface UseChatStreamReturn {
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

interface StreamState {
  messages: Message[];
  session: Session | undefined;
  chatState: ChatState;
  sessionLoadError: string | undefined;
  tokenState: TokenState;
  notifications: NotificationEvent[];
}

type StreamAction =
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

const initialTokenState: TokenState = {
  inputTokens: 0,
  outputTokens: 0,
  totalTokens: 0,
  accumulatedInputTokens: 0,
  accumulatedOutputTokens: 0,
  accumulatedTotalTokens: 0,
};

const initialState: StreamState = {
  messages: [],
  session: undefined,
  chatState: ChatState.Idle,
  sessionLoadError: undefined,
  tokenState: initialTokenState,
  notifications: [],
};

function streamReducer(state: StreamState, action: StreamAction): StreamState {
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

function pushMessage(currentMessages: Message[], incomingMsg: Message): Message[] {
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

function prefersReducedMotion(): boolean {
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

const REDUCED_MOTION_BATCH_INTERVAL = 1000;

async function streamFromResponse(
  stream: AsyncIterable<MessageEvent>,
  initialMessages: Message[],
  dispatch: React.Dispatch<StreamAction>,
  onFinish: (error?: string) => void,
  sessionId: string
): Promise<void> {
  let currentMessages = initialMessages;
  const reduceMotion = prefersReducedMotion();
  let latestTokenState: TokenState | null = null;
  let latestChatState: ChatState = ChatState.Streaming;
  let lastBatchUpdate = Date.now();
  let hasPendingUpdate = false;

  const flushBatchedUpdates = () => {
    if (reduceMotion && hasPendingUpdate) {
      if (latestTokenState) {
        dispatch({ type: 'SET_TOKEN_STATE', payload: latestTokenState });
      }
      dispatch({ type: 'SET_MESSAGES', payload: currentMessages });
      dispatch({ type: 'SET_CHAT_STATE', payload: latestChatState });
      hasPendingUpdate = false;
      lastBatchUpdate = Date.now();
    }
  };

  const maybeUpdateUI = (tokenState: TokenState, chatState: ChatState, forceImmediate = false) => {
    if (!reduceMotion) {
      dispatch({ type: 'SET_TOKEN_STATE', payload: tokenState });
      dispatch({ type: 'SET_MESSAGES', payload: currentMessages });
      dispatch({ type: 'SET_CHAT_STATE', payload: chatState });
    } else if (forceImmediate) {
      dispatch({ type: 'SET_TOKEN_STATE', payload: tokenState });
      dispatch({ type: 'SET_MESSAGES', payload: currentMessages });
      dispatch({ type: 'SET_CHAT_STATE', payload: chatState });
      hasPendingUpdate = false;
      lastBatchUpdate = Date.now();
    } else {
      latestTokenState = tokenState;
      latestChatState = chatState;
      hasPendingUpdate = true;
      const now = Date.now();
      if (now - lastBatchUpdate >= REDUCED_MOTION_BATCH_INTERVAL) {
        flushBatchedUpdates();
      }
    }
  };

  try {
    for await (const event of stream) {
      switch (event.type) {
        case 'Message': {
          const msg = event.message;
          currentMessages = pushMessage(currentMessages, msg);

          const hasToolConfirmation = msg.content.some(
            (content) =>
              content.type === 'actionRequired' && content.data.actionType === 'toolConfirmation'
          );

          const hasElicitation = msg.content.some(
            (content) =>
              content.type === 'actionRequired' && content.data.actionType === 'elicitation'
          );

          if (hasToolConfirmation || hasElicitation) {
            maybeUpdateUI(event.token_state, ChatState.WaitingForUserInput, true);
          } else if (getCompactingMessage(msg)) {
            maybeUpdateUI(event.token_state, ChatState.Compacting);
          } else if (getThinkingMessage(msg)) {
            maybeUpdateUI(event.token_state, ChatState.Thinking);
          } else {
            maybeUpdateUI(event.token_state, ChatState.Streaming);
          }
          break;
        }
        case 'Error': {
          flushBatchedUpdates();
          onFinish('Stream error: ' + event.error);
          return;
        }
        case 'Finish': {
          flushBatchedUpdates();
          onFinish();
          return;
        }
        case 'ModelChange': {
          break;
        }
        case 'UpdateConversation': {
          currentMessages = event.conversation;
          if (!reduceMotion) {
            dispatch({ type: 'SET_MESSAGES', payload: event.conversation });
          } else {
            hasPendingUpdate = true;
          }
          break;
        }
        case 'Notification': {
          dispatch({ type: 'ADD_NOTIFICATION', payload: event as NotificationEvent });
          maybeHandlePlatformEvent(event.message, sessionId);
          break;
        }
        case 'Ping':
          break;
      }
    }

    flushBatchedUpdates();
    onFinish();
  } catch (error) {
    flushBatchedUpdates();
    if (error instanceof Error && error.name !== 'AbortError') {
      onFinish('Stream error: ' + errorMessage(error));
    }
  }
}

export function useChatStream({
  sessionId,
  onStreamFinish,
  onSessionLoaded,
}: UseChatStreamProps): UseChatStreamReturn {
  const [state, dispatch] = useReducer(streamReducer, initialState);

  // Refs for values needed in callbacks without causing re-renders
  const abortControllerRef = useRef<AbortController | null>(null);
  const lastInteractionTimeRef = useRef<number>(Date.now());
  const namePollingRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Ref to access latest state in callbacks (avoids stale closures)
  const stateRef = useRef(state);
  stateRef.current = state;

  useEffect(() => {
    return () => {
      if (namePollingRef.current) {
        clearTimeout(namePollingRef.current);
        namePollingRef.current = null;
      }
    };
  }, [sessionId]);

  useEffect(() => {
    if (state.session) {
      resultsCache.set(sessionId, { session: state.session, messages: state.messages });
    }
  }, [sessionId, state.session, state.messages]);

  const onFinish = useCallback(
    async (error?: string): Promise<void> => {
      if (namePollingRef.current) {
        clearTimeout(namePollingRef.current);
        namePollingRef.current = null;
      }

      dispatch({ type: 'STREAM_FINISH', payload: error });

      const timeSinceLastInteraction = Date.now() - lastInteractionTimeRef.current;
      if (!error && timeSinceLastInteraction > 60000) {
        window.electron.showNotification({
          title: 'goose finished the task.',
          body: 'Click here to expand.',
        });
      }

      const isNewSession = sessionId && sessionId.match(/^\d{8}_\d{6}$/);
      if (isNewSession) {
        window.dispatchEvent(new CustomEvent(AppEvents.MESSAGE_STREAM_FINISHED));
      }

      // Refresh session name after each reply for the first 3 user messages
      // The backend regenerates the name after each of the first 3 user messages
      // to refine it as more context becomes available
      if (!error && sessionId) {
        const currentState = stateRef.current;
        const userMessageCount = currentState.messages.filter((m) => m.role === 'user').length;

        // Only refresh for the first 3 user messages
        if (userMessageCount <= 3) {
          try {
            const response = await getSession({
              path: { session_id: sessionId },
              throwOnError: true,
            });
            if (response.data?.name) {
              dispatch({
                type: 'SET_SESSION',
                payload: currentState.session
                  ? { ...currentState.session, name: response.data.name }
                  : undefined,
              });
              // Notify sidebar of the name change
              window.dispatchEvent(
                new CustomEvent(AppEvents.SESSION_RENAMED, {
                  detail: { sessionId, newName: response.data.name },
                })
              );
            }
          } catch (refreshError) {
            // Silently fail - this is a nice-to-have feature
            console.warn('Failed to refresh session name:', refreshError);
          }
        }
      }

      onStreamFinish();
    },
    [onStreamFinish, sessionId]
  );

  // Load session on mount or sessionId change
  useEffect(() => {
    if (!sessionId) return;

    const cached = resultsCache.get(sessionId);
    if (cached) {
      dispatch({
        type: 'SESSION_LOADED',
        payload: {
          session: cached.session,
          messages: cached.messages,
          tokenState: {
            inputTokens: cached.session?.input_tokens ?? 0,
            outputTokens: cached.session?.output_tokens ?? 0,
            totalTokens: cached.session?.total_tokens ?? 0,
            accumulatedInputTokens: cached.session?.accumulated_input_tokens ?? 0,
            accumulatedOutputTokens: cached.session?.accumulated_output_tokens ?? 0,
            accumulatedTotalTokens: cached.session?.accumulated_total_tokens ?? 0,
          },
        },
      });
      onSessionLoaded?.();
      return;
    }

    dispatch({ type: 'RESET_FOR_NEW_SESSION' });

    let cancelled = false;
    let loadFinished = false;

    const showExtensionEstimate = async () => {
      try {
        const response = await getSessionExtensions({
          path: { session_id: sessionId },
          throwOnError: true,
        });

        if (cancelled || loadFinished) {
          return;
        }

        const extensions = response.data?.extensions ?? [];
        if (extensions.length === 0) {
          return;
        }

        const extensionNames = extensions.map((ext) => ext.name);
        const { totalMs, perExtensionMs } = estimateExtensionLoadTimes(extensionNames);
        const statuses = extensionNames.map((name) => ({
          name,
          status: 'loading' as const,
          estimatedMs: perExtensionMs[name],
        }));

        toastService.extensionLoading(statuses, extensionNames.length, false, totalMs);
      } catch {
        // best-effort estimate; ignore failures
      }
    };

    void showExtensionEstimate();

    (async () => {
      try {
        const response = await resumeAgent({
          body: {
            session_id: sessionId,
            load_model_and_extensions: true,
          },
          throwOnError: true,
        });

        if (cancelled) {
          return;
        }
        loadFinished = true;

        const resumeData = response.data;
        const loadedSession = resumeData?.session;
        const extensionResults = resumeData?.extension_results;

        showExtensionLoadResults(extensionResults);
        window.dispatchEvent(new CustomEvent(AppEvents.SESSION_EXTENSIONS_LOADED));

        dispatch({
          type: 'SESSION_LOADED',
          payload: {
            session: loadedSession!,
            messages: loadedSession?.conversation || [],
            tokenState: {
              inputTokens: loadedSession?.input_tokens ?? 0,
              outputTokens: loadedSession?.output_tokens ?? 0,
              totalTokens: loadedSession?.total_tokens ?? 0,
              accumulatedInputTokens: loadedSession?.accumulated_input_tokens ?? 0,
              accumulatedOutputTokens: loadedSession?.accumulated_output_tokens ?? 0,
              accumulatedTotalTokens: loadedSession?.accumulated_total_tokens ?? 0,
            },
          },
        });

        listApps({
          throwOnError: true,
          query: { session_id: sessionId },
        }).catch((err) => {
          console.warn('Failed to populate apps cache:', err);
        });

        onSessionLoaded?.();
      } catch (error) {
        if (cancelled) return;
        loadFinished = true;

        dispatch({ type: 'STREAM_ERROR', payload: errorMessage(error) });
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [sessionId, onSessionLoaded]);

  const handleSubmit = useCallback(
    async (input: UserInput) => {
      const { msg: userMessage, images } = input;
      const currentState = stateRef.current;

      // Guard: Don't submit if session hasn't been loaded yet
      if (!currentState.session || currentState.chatState === ChatState.LoadingConversation) {
        return;
      }

      const hasExistingMessages = currentState.messages.length > 0;
      const hasNewMessage = userMessage.trim().length > 0 || images.length > 0;

      // Don't submit if there's no message and no conversation to continue
      if (!hasNewMessage && !hasExistingMessages) {
        return;
      }

      lastInteractionTimeRef.current = Date.now();

      // Emit session-created event for first message in a new session
      if (!hasExistingMessages && hasNewMessage) {
        window.dispatchEvent(new CustomEvent(AppEvents.SESSION_CREATED));

        // Start polling for session name update during streaming
        // The backend generates the name in parallel with the response
        const pollForName = async (attempts = 0) => {
          if (attempts >= 20) return; // Max 20 attempts (10 seconds)

          try {
            const response = await getSession({
              path: { session_id: sessionId },
              throwOnError: true,
            });
            const currentState = stateRef.current;
            const currentName = currentState.session?.name;
            const newName = response.data?.name;

            // Check if name has changed from the initial name
            if (newName && newName !== currentName) {
              dispatch({
                type: 'SET_SESSION',
                payload: currentState.session
                  ? { ...currentState.session, name: newName }
                  : undefined,
              });
              window.dispatchEvent(
                new CustomEvent(AppEvents.SESSION_RENAMED, {
                  detail: { sessionId, newName },
                })
              );
              return; // Stop polling once name is updated
            }
          } catch {
            // Silently continue polling
          }

          // Continue polling if still streaming
          const latestState = stateRef.current;
          if (
            latestState.chatState === ChatState.Streaming ||
            latestState.chatState === ChatState.Thinking ||
            latestState.chatState === ChatState.Compacting
          ) {
            namePollingRef.current = setTimeout(() => pollForName(attempts + 1), 500);
          }
        };

        // Start polling after a short delay to give backend time to start name generation
        namePollingRef.current = setTimeout(() => pollForName(0), 1000);
      }

      const newMessage = hasNewMessage
        ? createUserMessage(userMessage, images)
        : currentState.messages[currentState.messages.length - 1];
      const currentMessages = hasNewMessage
        ? [...currentState.messages, newMessage]
        : [...currentState.messages];

      if (hasNewMessage) {
        dispatch({ type: 'SET_MESSAGES', payload: currentMessages });
      }

      dispatch({ type: 'START_STREAMING' });
      abortControllerRef.current = new AbortController();

      try {
        const { stream } = await reply({
          body: {
            session_id: sessionId,
            user_message: newMessage,
            ...(hasExistingMessages && { conversation_so_far: currentState.messages }),
          },
          throwOnError: true,
          signal: abortControllerRef.current.signal,
        });

        await streamFromResponse(stream, currentMessages, dispatch, onFinish, sessionId);
      } catch (error) {
        // AbortError is expected when user stops streaming
        if (error instanceof Error && error.name === 'AbortError') {
          // Silently handle abort
        } else {
          // Unexpected error during fetch setup (streamFromResponse handles its own errors)
          onFinish('Submit error: ' + errorMessage(error));
        }
      }
    },
    [sessionId, onFinish]
  );

  const submitElicitationResponse = useCallback(
    async (elicitationId: string, userData: Record<string, unknown>) => {
      const currentState = stateRef.current;

      if (!currentState.session || currentState.chatState === ChatState.LoadingConversation) {
        return;
      }

      lastInteractionTimeRef.current = Date.now();

      const responseMessage = createElicitationResponseMessage(elicitationId, userData);
      const currentMessages = [...currentState.messages, responseMessage];

      dispatch({ type: 'SET_MESSAGES', payload: currentMessages });
      dispatch({ type: 'START_STREAMING' });
      abortControllerRef.current = new AbortController();

      try {
        const { stream } = await reply({
          body: {
            session_id: sessionId,
            user_message: responseMessage,
          },
          throwOnError: true,
          signal: abortControllerRef.current.signal,
        });

        await streamFromResponse(stream, currentMessages, dispatch, onFinish, sessionId);
      } catch (error) {
        if (error instanceof Error && error.name === 'AbortError') {
          // Silently handle abort
        } else {
          onFinish('Submit error: ' + errorMessage(error));
        }
      }
    },
    [sessionId, onFinish]
  );

  const setRecipeUserParams = useCallback(
    async (user_recipe_values: Record<string, string>) => {
      const currentState = stateRef.current;

      if (currentState.session) {
        await updateSessionUserRecipeValues({
          path: {
            session_id: sessionId,
          },
          body: {
            userRecipeValues: user_recipe_values,
          },
          throwOnError: true,
        });
        // TODO(Douwe): get this from the server instead of emulating it here
        dispatch({
          type: 'SET_SESSION',
          payload: {
            ...currentState.session,
            user_recipe_values,
          },
        });
      } else {
        dispatch({
          type: 'SET_SESSION_LOAD_ERROR',
          payload: "can't call setRecipeParams without a session",
        });
      }
    },
    [sessionId]
  );

  useEffect(() => {
    // This should happen on the server when the session is loaded or changed
    // use session.id to support changing of sessions rather than depending on the
    // stable sessionId.
    if (state.session) {
      updateFromSession({
        body: {
          session_id: state.session.id,
        },
        throwOnError: true,
      });
    }
  }, [state.session]);

  const stopStreaming = useCallback(() => {
    abortControllerRef.current?.abort();
    dispatch({ type: 'SET_CHAT_STATE', payload: ChatState.Idle });
    lastInteractionTimeRef.current = Date.now();
  }, []);

  const onMessageUpdate = useCallback(
    async (messageId: string, newContent: string, editType: 'fork' | 'edit' = 'fork') => {
      const currentState = stateRef.current;

      dispatch({ type: 'SET_CHAT_STATE', payload: ChatState.Thinking });

      try {
        const { forkSession } = await import('../api');
        const message = currentState.messages.find((m) => m.id === messageId);

        if (!message) {
          throw new Error(`Message with id ${messageId} not found in current messages`);
        }

        const response = await forkSession({
          path: {
            session_id: sessionId,
          },
          body: {
            timestamp: message.created,
            truncate: true,
            copy: editType === 'fork',
          },
          throwOnError: true,
        });

        const targetSessionId = response.data?.sessionId;
        if (!targetSessionId) {
          throw new Error('No session ID returned from fork');
        }

        if (editType === 'fork') {
          dispatch({ type: 'SET_CHAT_STATE', payload: ChatState.Idle });
          const event = new CustomEvent(AppEvents.SESSION_FORKED, {
            detail: {
              newSessionId: targetSessionId,
              shouldStartAgent: true,
              editedMessage: newContent,
            },
          });
          window.dispatchEvent(event);
          window.electron.logInfo(`Dispatched session-forked event for session ${targetSessionId}`);
        } else {
          const { getSession } = await import('../api');
          const sessionResponse = await getSession({
            path: { session_id: targetSessionId },
            throwOnError: true,
          });

          if (sessionResponse.data?.conversation) {
            const truncatedMessages = [...sessionResponse.data.conversation];
            const updatedUserMessage = createUserMessage(newContent);

            for (const content of message.content) {
              if (content.type === 'image') {
                updatedUserMessage.content.push(content);
              }
            }

            const messagesForUI = [...truncatedMessages, updatedUserMessage];
            dispatch({ type: 'SET_MESSAGES', payload: messagesForUI });
            dispatch({ type: 'START_STREAMING' });

            abortControllerRef.current = new AbortController();

            try {
              const { stream } = await reply({
                body: {
                  session_id: targetSessionId,
                  user_message: updatedUserMessage,
                  conversation_so_far: truncatedMessages,
                },
                throwOnError: true,
                signal: abortControllerRef.current.signal,
              });

              await streamFromResponse(stream, messagesForUI, dispatch, onFinish, targetSessionId);
            } catch (error) {
              if (error instanceof Error && error.name === 'AbortError') {
                dispatch({ type: 'SET_CHAT_STATE', payload: ChatState.Idle });
              } else {
                throw error;
              }
            }
          } else {
            await handleSubmit({ msg: newContent, images: [] });
          }
        }
      } catch (error) {
        dispatch({ type: 'SET_CHAT_STATE', payload: ChatState.Idle });
        const errorMsg = errorMessage(error);
        console.error('Failed to edit message:', error);
        const { toastError } = await import('../toasts');
        toastError({
          title: 'Failed to edit message',
          msg: errorMsg,
        });
      }
    },
    [sessionId, handleSubmit, onFinish]
  );

  const setChatState = useCallback((newState: ChatState) => {
    dispatch({ type: 'SET_CHAT_STATE', payload: newState });
  }, []);

  const cached = resultsCache.get(sessionId);
  const maybe_cached_messages = state.session ? state.messages : cached?.messages || [];
  const maybe_cached_session = state.session ?? cached?.session;

  const notificationsMap = useMemo(() => {
    return state.notifications.reduce((map, notification) => {
      const key = notification.request_id;
      if (!map.has(key)) {
        map.set(key, []);
      }
      map.get(key)!.push(notification);
      return map;
    }, new Map<string, NotificationEvent[]>());
  }, [state.notifications]);

  return {
    sessionLoadError: state.sessionLoadError,
    messages: maybe_cached_messages,
    session: maybe_cached_session,
    chatState: state.chatState,
    setChatState,
    handleSubmit,
    submitElicitationResponse,
    stopStreaming,
    setRecipeUserParams,
    tokenState: state.tokenState,
    notifications: notificationsMap,
    onMessageUpdate,
  };
}
