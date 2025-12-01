import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { ChatState } from '../types/chatState';
import {
  Message,
  Session,
  TokenState,
  updateFromSession,
  updateSessionUserRecipeValues,
} from '../api';
import { createUserMessage, NotificationEvent } from '../types/message';
import { errorMessage } from '../utils/conversionUtils';
import { useSessionWorkerContext } from '../contexts/SessionWorkerContext';

interface UseChatStreamWorkerProps {
  sessionId: string;
  onStreamFinish: () => void;
  onSessionLoaded?: () => void;
}

interface UseChatStreamWorkerReturn {
  session?: Session;
  messages: Message[];
  chatState: ChatState;
  handleSubmit: (userMessage: string) => Promise<void>;
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

/**
 * Worker-based version of useChatStream
 * Delegates streaming to Web Worker for better performance and scalability
 * Maintains the same interface as useChatStream for drop-in replacement
 */
export function useChatStreamWorker({
  sessionId,
  onStreamFinish,
  onSessionLoaded,
}: UseChatStreamWorkerProps): UseChatStreamWorkerReturn {
  const worker = useSessionWorkerContext();

  const [messages, setMessages] = useState<Message[]>([]);
  const messagesRef = useRef<Message[]>([]);
  const [session, setSession] = useState<Session>();
  const [sessionLoadError, setSessionLoadError] = useState<string>();
  const [chatState, setChatState] = useState<ChatState>(ChatState.Idle);
  const [tokenState, setTokenState] = useState<TokenState>({
    inputTokens: 0,
    outputTokens: 0,
    totalTokens: 0,
    accumulatedInputTokens: 0,
    accumulatedOutputTokens: 0,
    accumulatedTotalTokens: 0,
  });
  const [notifications, setNotifications] = useState<NotificationEvent[]>([]);

  const updateMessages = useCallback((newMessages: Message[]) => {
    setMessages(newMessages);
    messagesRef.current = newMessages;
  }, []);

  const onFinish = useCallback(
    async (error?: string): Promise<void> => {
      if (error) {
        setSessionLoadError(error);
      }

      const isNewSession = sessionId && sessionId.match(/^\d{8}_\d{6}$/);
      if (isNewSession) {
        window.dispatchEvent(new CustomEvent('message-stream-finished'));
      }

      setChatState(ChatState.Idle);
      onStreamFinish();
    },
    [onStreamFinish, sessionId]
  );

  // Load session on mount or sessionId change
  useEffect(() => {
    if (!sessionId) {
      return;
    }

    let cancelled = false;

    (async () => {
      try {
        await worker.waitForReady();
        const existingState = await worker.getSessionState(sessionId);

        if (cancelled) {
          return;
        }

        if (existingState && existingState.streamState !== 'loading') {
          setSession(existingState.session);
          updateMessages(existingState.messages);
          setTokenState(existingState.tokenState);
          setNotifications(existingState.notifications);

          // Set chat state based on worker's stream state
          if (existingState.streamState === 'streaming') {
            console.log('[useChatStreamWorker] Restoring to streaming state');
            setChatState(ChatState.Streaming);
          } else {
            console.log('[useChatStreamWorker] Restoring to idle state');
            setChatState(ChatState.Idle);
          }

          onSessionLoaded?.();
        } else if (existingState && existingState.streamState === 'loading') {
          updateMessages([]);
          setSession(undefined);
          setSessionLoadError(undefined);
          setChatState(ChatState.LoadingConversation);
        } else {
          // Show loading state
          updateMessages([]);
          setSession(undefined);
          setSessionLoadError(undefined);
          setChatState(ChatState.LoadingConversation);

          const state = await worker.loadSession(sessionId);

          if (cancelled) {
            return;
          }

          if (state) {
            setSession(state.session);
            updateMessages(state.messages);
            setTokenState(state.tokenState);
            setNotifications(state.notifications);
            setChatState(ChatState.Idle);
            onSessionLoaded?.();
          } else {
            console.error('[useChatStreamWorker] No state returned after loadSession');
          }
        }
      } catch (error) {
        if (cancelled) {
          return;
        }

        console.error('[useChatStreamWorker] Failed to load session:', error);
        setSessionLoadError(errorMessage(error));
        setChatState(ChatState.Idle);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [sessionId, worker, updateMessages, onSessionLoaded]);

  // Subscribe to session updates from worker
  useEffect(() => {
    if (!sessionId) return;

    return worker.subscribeToSession(sessionId, (update) => {
      console.log('[useChatStreamWorker] Received update:', update);

      // Update session object if provided
      if (update.session) {
        setSession(update.session);
      }

      if (update.messages) {
        // Always update messages from worker - it's the source of truth
        updateMessages(update.messages);
      }

      if (update.tokenState) {
        setTokenState(update.tokenState);
      }

      if (update.notifications) {
        setNotifications(update.notifications);
      }

      if (update.streamState === 'streaming') {
        setChatState(ChatState.Streaming);
      } else if (update.streamState === 'idle') {
        setChatState(ChatState.Idle);
        onFinish();
      } else if (update.streamState === 'error') {
        setChatState(ChatState.Idle);
        onFinish(update.error);
      }
    });
  }, [sessionId, worker, updateMessages, onFinish]);

  const handleSubmit = useCallback(
    async (userMessage: string) => {
      // Guard: Don't submit if session hasn't been loaded yet
      if (!session || chatState === ChatState.LoadingConversation) {
        return;
      }

      const hasExistingMessages = messagesRef.current.length > 0;
      const hasNewMessage = userMessage.trim().length > 0;

      // Don't submit if there's no message and no conversation to continue
      if (!hasNewMessage && !hasExistingMessages) {
        return;
      }

      // Emit session-created event for first message in a new session
      if (!hasExistingMessages && hasNewMessage) {
        window.dispatchEvent(new CustomEvent('session-created'));
      }

      // Build message list: add new message if provided, otherwise continue with existing
      const currentMessages = hasNewMessage
        ? [...messagesRef.current, createUserMessage(userMessage)]
        : [...messagesRef.current];

      setChatState(ChatState.Streaming);

      try {
        await worker.startStream(sessionId, userMessage, currentMessages);
      } catch (error) {
        console.error('[useChatStreamWorker] Stream error:', error);
        await onFinish(errorMessage(error));
      }
    },
    [sessionId, session, chatState, onFinish, worker]
  );

  const setRecipeUserParams = useCallback(
    async (user_recipe_values: Record<string, string>) => {
      if (session) {
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
        const updatedSession = {
          ...session,
          user_recipe_values,
        };
        setSession(updatedSession);
        await worker.updateSession(sessionId, updatedSession);
      } else {
        setSessionLoadError("can't call setRecipeParams without a session");
      }
    },
    [sessionId, session, worker]
  );

  useEffect(() => {
    // This should happen on the server when the session is loaded or changed
    // use session.id to support changing of sessions rather than depending on the
    // stable sessionId.
    if (session) {
      updateFromSession({
        body: {
          session_id: session.id,
        },
        throwOnError: true,
      });
    }
  }, [session]);

  const stopStreaming = useCallback(() => {
    worker.stopStream(sessionId);
    setChatState(ChatState.Idle);
  }, [sessionId, worker]);

  const onMessageUpdate = useCallback(
    async (messageId: string, newContent: string, editType: 'fork' | 'edit' = 'fork') => {
      try {
        const { editMessage } = await import('../api');
        const message = messagesRef.current.find((m) => m.id === messageId);

        if (!message) {
          throw new Error(`Message with id ${messageId} not found in current messages`);
        }

        const response = await editMessage({
          path: {
            session_id: sessionId,
          },
          body: {
            timestamp: message.created,
            editType,
          },
          throwOnError: true,
        });

        const targetSessionId = response.data?.sessionId;
        if (!targetSessionId) {
          throw new Error('No session ID returned from edit_message');
        }

        if (editType === 'fork') {
          const event = new CustomEvent('session-forked', {
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
            updateMessages(sessionResponse.data.conversation);
          }
          await handleSubmit(newContent);
        }
      } catch (error) {
        const errorMsg = errorMessage(error);
        console.error('Failed to edit message:', error);
        const { toastError } = await import('../toasts');
        toastError({
          title: 'Failed to edit message',
          msg: errorMsg,
        });
      }
    },
    [sessionId, handleSubmit, updateMessages]
  );

  const notificationsMap = useMemo(() => {
    return notifications.reduce((map, notification) => {
      const key = notification.request_id;
      if (!map.has(key)) {
        map.set(key, []);
      }
      map.get(key)!.push(notification);
      return map;
    }, new Map<string, NotificationEvent[]>());
  }, [notifications]);

  return {
    sessionLoadError,
    messages,
    session,
    chatState,
    handleSubmit,
    stopStreaming,
    setRecipeUserParams,
    tokenState,
    notifications: notificationsMap,
    onMessageUpdate,
  };
}
