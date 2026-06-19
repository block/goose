import { useCallback, useEffect, useMemo, useRef } from 'react';
import { defineMessages, useIntl } from '../i18n';
import { v7 as uuidv7 } from 'uuid';
import { AppEvents } from '../constants/events';
import { ChatState } from '../types/chatState';

import {
  Message,
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
import {
  acpForkSession,
  acpLoadSession,
  acpTruncateSessionConversation,
  sessionInfoToSession,
} from '../acp/sessions';
import { acpChatSessionStore, useAcpChatSessionSnapshot } from '../acp/chatSessionStore';

const initialTokenState: TokenState = {
  inputTokens: 0,
  outputTokens: 0,
  totalTokens: 0,
  accumulatedInputTokens: 0,
  accumulatedOutputTokens: 0,
  accumulatedTotalTokens: 0,
};

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
  const acpSnapshot = useAcpChatSessionSnapshot(sessionId);
  const messages = acpSnapshot?.messages ?? [];
  const session = acpSnapshot?.session;
  const chatState = acpSnapshot?.chatState ?? ChatState.LoadingConversation;
  const sessionLoadError = acpSnapshot?.sessionLoadError;
  const tokenState = acpSnapshot?.tokenState ?? initialTokenState;
  const notifications = acpSnapshot?.notifications ?? [];

  const snapshotRef = useRef(acpSnapshot);
  snapshotRef.current = acpSnapshot;

  useEffect(() => {
    const handleSessionRenamed = (event: Event) => {
      const { sessionId: renamedSessionId, newName } = (
        event as CustomEvent<{ sessionId: string; newName: string }>
      ).detail;

      if (renamedSessionId !== sessionId) {
        return;
      }

      const currentSession =
        snapshotRef.current?.session ?? acpChatSessionStore.getSnapshot(sessionId)?.session;
      if (!currentSession || currentSession.name === newName) {
        return;
      }

      const updatedSession = { ...currentSession, name: newName };
      acpChatSessionStore.setSessionMetadata(sessionId, updatedSession);
    };

    window.addEventListener(AppEvents.SESSION_RENAMED, handleSessionRenamed);
    return () => window.removeEventListener(AppEvents.SESSION_RENAMED, handleSessionRenamed);
  }, [sessionId]);

  const onFinish = useCallback(
    async (error?: string): Promise<void> => {
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
            ...(snapshotRef.current?.messages ??
              acpChatSessionStore.getSnapshot(targetSessionId)?.messages ??
              []),
            createAcpCreditsExhaustedMessage(creditsExhaustedError),
          ];
          acpChatSessionStore.setMessages(targetSessionId, messages);
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
      window.dispatchEvent(
        new CustomEvent(AppEvents.SESSION_EXTENSIONS_LOADED, { detail: { sessionId } })
      );
      onSessionLoaded?.();
      return;
    }

    acpChatSessionStore.startSessionLoad(sessionId);

    let cancelled = false;
    let loadSettled = false;

    (async () => {
      try {
        const { sessionInfo, meta } = await acpLoadSession(sessionId);

        if (cancelled) {
          return;
        }

        const loadedSession = sessionInfoToSession(sessionInfo, meta);
        const extensionResults = meta.extensionResults;

        showExtensionLoadResults(extensionResults);
        window.dispatchEvent(
          new CustomEvent(AppEvents.SESSION_EXTENSIONS_LOADED, { detail: { sessionId } })
        );

        acpChatSessionStore.finishSessionLoad(sessionId, loadedSession);

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
        acpChatSessionStore.failSessionLoad(sessionId, loadError);
      } finally {
        loadSettled = true;
      }
    })();

    return () => {
      cancelled = true;
      if (!loadSettled) {
        acpChatSessionStore.setChatState(sessionId, ChatState.Idle);
      }
    };
  }, [sessionId, onSessionLoaded]);

  const handleSubmit = useCallback(
    async (input: UserInput) => {
      const { msg: userMessage, images } = input;
      const currentSnapshot = snapshotRef.current ?? acpChatSessionStore.getSnapshot(sessionId);

      if (
        !currentSnapshot?.session ||
        currentSnapshot.chatState === ChatState.LoadingConversation ||
        currentSnapshot.chatState === ChatState.Streaming ||
        currentSnapshot.chatState === ChatState.Thinking ||
        currentSnapshot.chatState === ChatState.Compacting
      ) {
        return;
      }

      const currentMessages = currentSnapshot.messages;
      const hasExistingMessages = currentMessages.length > 0;
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
        : currentMessages[currentMessages.length - 1];
      const messagesForStore = clearsConversation
        ? []
        : hasNewMessage
          ? [...currentMessages, newMessage]
          : [...currentMessages];

      if (clearsConversation || hasNewMessage) {
        acpChatSessionStore.setMessages(sessionId, messagesForStore);
      }

      await submitToAcpSession(sessionId, newMessage);
    },
    [sessionId, submitToAcpSession]
  );

  const submitElicitationResponse = useCallback(
    async (elicitationId: string, userData: Record<string, unknown>) => {
      const currentSnapshot = snapshotRef.current ?? acpChatSessionStore.getSnapshot(sessionId);

      if (!currentSnapshot?.session || currentSnapshot.chatState === ChatState.LoadingConversation) {
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
      const currentSession =
        snapshotRef.current?.session ?? acpChatSessionStore.getSnapshot(sessionId)?.session;

      if (currentSession) {
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
          ...currentSession,
          user_recipe_values,
        };
        acpChatSessionStore.setSessionMetadata(sessionId, updatedSession);
      } else {
        acpChatSessionStore.setSessionLoadError(
          sessionId,
          "can't call setRecipeParams without a session"
        );
      }
    },
    [sessionId]
  );

  useEffect(() => {
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
      return;
    }

    acpChatSessionStore.setChatState(sessionId, ChatState.Idle);
  }, [sessionId]);

  const onMessageUpdate = useCallback(
    async (messageId: string, newContent: string, editType: 'fork' | 'edit' = 'fork') => {
      const currentSnapshot = snapshotRef.current ?? acpChatSessionStore.getSnapshot(sessionId);

      acpChatSessionStore.setChatState(sessionId, ChatState.Thinking);

      try {
        const currentMessages = currentSnapshot?.messages ?? [];
        const message = currentMessages.find((m) => m.id === messageId);

        if (!message) {
          throw new Error(`Message with id ${messageId} not found in current messages`);
        }

        if (editType === 'fork') {
          const targetSessionId = await acpForkSession(sessionId, message.created);

          acpChatSessionStore.setChatState(sessionId, ChatState.Idle);
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

          const truncatedMessages = currentMessages.filter((m) => m.created < message.created);
          const updatedUserMessage = createUserMessage(newContent);

          for (const content of message.content) {
            if (content.type === 'image') {
              updatedUserMessage.content.push(content);
            }
          }

          const messagesForUI = [...truncatedMessages, updatedUserMessage];
          acpChatSessionStore.setMessages(sessionId, messagesForUI);

          await submitToAcpSession(sessionId, updatedUserMessage);
        }
      } catch (error) {
        acpChatSessionStore.setChatState(sessionId, ChatState.Idle);
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
    },
    [sessionId]
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
    setChatState,
    handleSubmit,
    submitElicitationResponse,
    stopStreaming,
    setRecipeUserParams,
    tokenState,
    notifications: notificationsMap,
    pauseQueueOnStop: true,
    onMessageUpdate,
  };
}
