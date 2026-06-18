import { useCallback, useEffect, useMemo, useReducer, useRef } from 'react';
import { defineMessages, useIntl } from '../i18n';
import { v7 as uuidv7 } from 'uuid';
import { AppEvents } from '../constants/events';
import { ChatState } from '../types/chatState';

import {
  Message,
  resumeAgent,
  Session,
  TokenState,
  updateFromSession,
  updateSessionUserRecipeValues,
  listApps,
} from '../api';

import { createUserMessage, NotificationEvent, UserInput } from '../types/message';
import { errorMessage } from '../utils/conversionUtils';
import { showExtensionLoadResults } from '../utils/extensionErrorUtils';
import type { UseChatSessionParams, UseChatSessionResult } from './useChatSessionTypes';
import { cancelAcpPermissionRequestsForSession } from '../acp/permissionRequests';
import {
  cancelAcpElicitationRequestsForSession,
  resolveAcpElicitationRequest,
} from '../acp/elicitationRequests';
import { parseAcpCreditsExhaustedError, type AcpCreditsExhaustedError } from '../acp/errors';
import { acpCancelPrompt, acpPromptSession } from '../acp/prompt';
import { acpForkSession, acpTruncateSessionConversation } from '../acp/sessions';
import { acpChatSessionStore, type AcpChatSessionSnapshot } from '../acp/chatSessionStore';

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
  | { type: 'SYNC_FROM_ACP_STORE'; payload: AcpChatSessionSnapshot }
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

    case 'SYNC_FROM_ACP_STORE':
      return {
        ...state,
        session: action.payload.session,
        messages: action.payload.messages,
        tokenState: action.payload.tokenState,
        notifications: action.payload.notifications,
        chatState: action.payload.chatState,
        sessionLoadError: action.payload.sessionLoadError,
      };

    case 'RESET_FOR_NEW_SESSION':
      return {
        ...state,
        messages: [],
        session: undefined,
        sessionLoadError: undefined,
        notifications: [],
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

function isClearCommand(message: string): boolean {
  return message.trim() === '/clear';
}

function createAcpCreditsExhaustedMessage(error: AcpCreditsExhaustedError): Message {
  return {
    id: uuidv7(),
    role: 'assistant',
    created: Math.floor(Date.now() / 1000),
    content: [
      {
        type: 'systemNotification',
        notificationType: 'creditsExhausted',
        msg: error.message,
        ...(error.url ? { data: { top_up_url: error.url } } : {}),
      },
    ],
    metadata: { userVisible: true, agentVisible: false },
  };
}

const i18n = defineMessages({
  notificationTitle: {
    id: 'chat.notification.taskComplete.title',
    defaultMessage: 'Goose finished the task.',
  },
  notificationBody: {
    id: 'chat.notification.taskComplete.body',
    defaultMessage: 'Click here to bring Goose back into focus.',
  },
});

export function useAcpChatSession({
  sessionId,
  onStreamFinish,
  onSessionLoaded,
}: UseChatSessionParams): UseChatSessionResult {
  const intl = useIntl();
  const [state, dispatch] = useReducer(streamReducer, initialState);

  // Ref to access latest state in callbacks (avoids stale closures)
  const stateRef = useRef(state);
  stateRef.current = state;

  useEffect(() => {
    if (!sessionId) {
      return;
    }

    const snapshot = acpChatSessionStore.getSnapshot(sessionId);
    if (snapshot) {
      dispatch({ type: 'SYNC_FROM_ACP_STORE', payload: snapshot });
    }

    return acpChatSessionStore.subscribe(sessionId, (nextSnapshot) => {
      dispatch({ type: 'SYNC_FROM_ACP_STORE', payload: nextSnapshot });
    });
  }, [sessionId]);

  useEffect(() => {
    const handleSessionRenamed = (event: Event) => {
      const { sessionId: renamedSessionId, newName } = (
        event as CustomEvent<{ sessionId: string; newName: string }>
      ).detail;

      if (renamedSessionId !== sessionId) {
        return;
      }

      const currentSession = stateRef.current.session;
      if (!currentSession || currentSession.name === newName) {
        return;
      }

      const updatedSession = { ...currentSession, name: newName };
      acpChatSessionStore.setSessionMetadata(sessionId, updatedSession);
      dispatch({ type: 'SET_SESSION', payload: updatedSession });
    };

    window.addEventListener(AppEvents.SESSION_RENAMED, handleSessionRenamed);
    return () => window.removeEventListener(AppEvents.SESSION_RENAMED, handleSessionRenamed);
  }, [sessionId]);

  const onFinish = useCallback(
    async (error?: string): Promise<void> => {
      acpChatSessionStore.setSessionLoadError(sessionId, error);
      acpChatSessionStore.setChatState(sessionId, ChatState.Idle);
      dispatch({ type: 'STREAM_FINISH', payload: error });

      if (!error) {
        try {
          const [notificationsEnabled, anyWindowFocused] = await Promise.all([
            window.electron.getSetting('enableNotifications'),
            window.electron.isAnyWindowFocused(),
          ]);
          if (notificationsEnabled === true && !anyWindowFocused) {
            window.electron.showNotification({
              title: intl.formatMessage(i18n.notificationTitle),
              body: intl.formatMessage(i18n.notificationBody),
            });
          }
        } catch (notifyError) {
          console.warn('Failed to show task completion notification:', notifyError);
        }
      }

      const isNewSession = sessionId && sessionId.match(/^\d{8}_\d{6}$/);
      if (isNewSession) {
        window.dispatchEvent(new CustomEvent(AppEvents.MESSAGE_STREAM_FINISHED));
      }

      onStreamFinish();
    },
    [intl, onStreamFinish, sessionId]
  );

  const submitToAcpSession = useCallback(
    async (targetSessionId: string, userMessage: Message) => {
      const promptAttemptId = uuidv7();
      acpChatSessionStore.startPromptAttempt(targetSessionId, promptAttemptId);

      try {
        await acpPromptSession(targetSessionId, userMessage);
        if (acpChatSessionStore.finishPromptAttemptIfCurrent(targetSessionId, promptAttemptId)) {
          onFinish();
        }
      } catch (error) {
        const creditsExhaustedError = parseAcpCreditsExhaustedError(error);
        if (creditsExhaustedError) {
          if (!acpChatSessionStore.isCurrentPromptAttempt(targetSessionId, promptAttemptId)) {
            return;
          }

          const messages = [
            ...stateRef.current.messages,
            createAcpCreditsExhaustedMessage(creditsExhaustedError),
          ];
          acpChatSessionStore.setMessages(targetSessionId, messages);
          dispatch({
            type: 'SET_MESSAGES',
            payload: messages,
          });
          if (acpChatSessionStore.finishPromptAttemptIfCurrent(targetSessionId, promptAttemptId)) {
            onFinish();
          }
          return;
        }

        const submitError = 'Submit error: ' + errorMessage(error);
        if (
          acpChatSessionStore.finishPromptAttemptIfCurrent(
            targetSessionId,
            promptAttemptId,
            submitError
          )
        ) {
          onFinish(submitError);
        }
      }
    },
    [onFinish]
  );

  // Load session on mount or sessionId change
  useEffect(() => {
    if (!sessionId) return;

    const cached = acpChatSessionStore.getSnapshot(sessionId);
    if (cached?.session) {
      dispatch({ type: 'SYNC_FROM_ACP_STORE', payload: cached });
      window.dispatchEvent(
        new CustomEvent(AppEvents.SESSION_EXTENSIONS_LOADED, { detail: { sessionId } })
      );
      onSessionLoaded?.();
      return;
    }

    dispatch({ type: 'RESET_FOR_NEW_SESSION' });

    let cancelled = false;

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

        const resumeData = response.data;
        const loadedSession = resumeData?.session;
        const extensionResults = resumeData?.extension_results;

        showExtensionLoadResults(extensionResults);
        window.dispatchEvent(
          new CustomEvent(AppEvents.SESSION_EXTENSIONS_LOADED, { detail: { sessionId } })
        );

        if (loadedSession) {
          const snapshot = acpChatSessionStore.setLoadedSession(sessionId, loadedSession);
          dispatch({ type: 'SYNC_FROM_ACP_STORE', payload: snapshot });
        }

        listApps({
          throwOnError: true,
          query: { session_id: sessionId },
        }).catch((err) => {
          console.warn('Failed to populate apps cache:', err);
        });

        onSessionLoaded?.();
      } catch (error) {
        if (cancelled) return;

        const loadError = errorMessage(error);
        acpChatSessionStore.setSessionLoadError(sessionId, loadError);
        acpChatSessionStore.setChatState(sessionId, ChatState.Idle);
        dispatch({ type: 'STREAM_ERROR', payload: loadError });
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

      if (
        !currentState.session ||
        currentState.chatState === ChatState.LoadingConversation ||
        currentState.chatState === ChatState.Streaming ||
        currentState.chatState === ChatState.Thinking ||
        currentState.chatState === ChatState.Compacting
      ) {
        return;
      }

      const hasExistingMessages = currentState.messages.length > 0;
      const hasNewMessage = userMessage.trim().length > 0 || images.length > 0;
      const clearsConversation = hasNewMessage && isClearCommand(userMessage);

      if (!hasNewMessage && !hasExistingMessages) {
        return;
      }

      // Emit session-created event for first message in a new session
      if (!hasExistingMessages && hasNewMessage) {
        window.dispatchEvent(new CustomEvent(AppEvents.SESSION_CREATED));
      }

      const newMessage = hasNewMessage
        ? createUserMessage(userMessage, images)
        : currentState.messages[currentState.messages.length - 1];
      const currentMessages = clearsConversation
        ? []
        : hasNewMessage
          ? [...currentState.messages, newMessage]
          : [...currentState.messages];

      if (clearsConversation || hasNewMessage) {
        acpChatSessionStore.setMessages(sessionId, currentMessages);
        dispatch({ type: 'SET_MESSAGES', payload: currentMessages });
      }

      acpChatSessionStore.setChatState(sessionId, ChatState.Streaming);
      dispatch({ type: 'START_STREAMING' });

      await submitToAcpSession(sessionId, newMessage);
    },
    [sessionId, submitToAcpSession]
  );

  const submitElicitationResponse = useCallback(
    async (elicitationId: string, userData: Record<string, unknown>) => {
      const currentState = stateRef.current;

      if (!currentState.session || currentState.chatState === ChatState.LoadingConversation) {
        return false;
      }

      if (!resolveAcpElicitationRequest(sessionId, elicitationId, userData)) {
        console.error('No pending ACP elicitation request found', { sessionId, elicitationId });
        return false;
      }

      return true;
    },
    [sessionId]
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
        const updatedSession = {
          ...currentState.session,
          user_recipe_values,
        };
        acpChatSessionStore.setSessionMetadata(sessionId, updatedSession);
        dispatch({ type: 'SET_SESSION', payload: updatedSession });
      } else {
        acpChatSessionStore.setSessionLoadError(
          sessionId,
          "can't call setRecipeParams without a session"
        );
        dispatch({
          type: 'SET_SESSION_LOAD_ERROR',
          payload: "can't call setRecipeParams without a session",
        });
      }
    },
    [sessionId]
  );

  useEffect(() => {
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
    const storedPromptAttemptId = acpChatSessionStore.getSnapshot(sessionId)?.activePromptAttemptId;
    const hasStoredAcpPrompt =
      storedPromptAttemptId !== null && storedPromptAttemptId !== undefined;

    if (hasStoredAcpPrompt) {
      acpChatSessionStore.clearActivePromptAttempt(sessionId);
      cancelAcpPermissionRequestsForSession(sessionId);
      cancelAcpElicitationRequestsForSession(sessionId);
      acpCancelPrompt(sessionId).catch((e) => {
        console.warn('Failed to cancel ACP prompt:', e);
      });
    }

    acpChatSessionStore.setChatState(sessionId, ChatState.Idle);
    dispatch({ type: 'SET_CHAT_STATE', payload: ChatState.Idle });
  }, [sessionId]);

  const onMessageUpdate = useCallback(
    async (messageId: string, newContent: string, editType: 'fork' | 'edit' = 'fork') => {
      const currentState = stateRef.current;

      acpChatSessionStore.setChatState(sessionId, ChatState.Thinking);
      dispatch({ type: 'SET_CHAT_STATE', payload: ChatState.Thinking });

      try {
        const message = currentState.messages.find((m) => m.id === messageId);

        if (!message) {
          throw new Error(`Message with id ${messageId} not found in current messages`);
        }

        if (editType === 'fork') {
          const targetSessionId = await acpForkSession(sessionId, message.created);

          acpChatSessionStore.setChatState(sessionId, ChatState.Idle);
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
          await acpTruncateSessionConversation(sessionId, message.created);

          const truncatedMessages = currentState.messages.filter(
            (m) => m.created < message.created
          );
          const updatedUserMessage = createUserMessage(newContent);

          for (const content of message.content) {
            if (content.type === 'image') {
              updatedUserMessage.content.push(content);
            }
          }

          const messagesForUI = [...truncatedMessages, updatedUserMessage];
          acpChatSessionStore.setMessages(sessionId, messagesForUI);
          acpChatSessionStore.setChatState(sessionId, ChatState.Streaming);
          dispatch({ type: 'SET_MESSAGES', payload: messagesForUI });
          dispatch({ type: 'START_STREAMING' });

          await submitToAcpSession(sessionId, updatedUserMessage);
        }
      } catch (error) {
        acpChatSessionStore.setChatState(sessionId, ChatState.Idle);
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
    [sessionId, submitToAcpSession]
  );

  const setChatState = useCallback(
    (newState: ChatState) => {
      acpChatSessionStore.setChatState(sessionId, newState);
      dispatch({ type: 'SET_CHAT_STATE', payload: newState });
    },
    [sessionId]
  );

  const cached = acpChatSessionStore.getSnapshot(sessionId);
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
    pauseQueueOnStop: true,
    onMessageUpdate,
  };
}
