import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { ChatState } from '../types/chatState';

import {
  Message,
  MessageEvent,
  reply,
  resumeAgent,
  Session,
  TokenState,
  updateFromSession,
  updateSessionUserRecipeValues,
} from '../api';

import {
  createUserMessage,
  createElicitationResponseMessage,
  getCompactingMessage,
  getThinkingMessage,
  NotificationEvent,
} from '../types/message';
import { errorMessage } from '../utils/conversionUtils';

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
  handleSubmit: (userMessage: string) => Promise<void>;
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

async function streamFromResponse(
  stream: AsyncIterable<MessageEvent>,
  initialMessages: Message[],
  updateMessages: (messages: Message[]) => void,
  updateTokenState: (tokenState: TokenState) => void,
  updateChatState: (state: ChatState) => void,
  updateNotifications: (notification: NotificationEvent) => void,
  onFinish: (error?: string) => void
): Promise<void> {
  let currentMessages = initialMessages;

  try {
    for await (const event of stream) {
      switch (event.type) {
        case 'Message': {
          const msg = event.message;
          currentMessages = pushMessage(currentMessages, msg);

          const hasToolConfirmation = msg.content.some(
            (content) => content.type === 'toolConfirmationRequest'
          );

          const hasElicitation = msg.content.some(
            (content) =>
              content.type === 'actionRequired' && content.data.actionType === 'elicitation'
          );

          if (hasToolConfirmation || hasElicitation) {
            updateChatState(ChatState.WaitingForUserInput);
          } else if (getCompactingMessage(msg)) {
            updateChatState(ChatState.Compacting);
          } else if (getThinkingMessage(msg)) {
            updateChatState(ChatState.Thinking);
          } else {
            updateChatState(ChatState.Streaming);
          }

          updateTokenState(event.token_state);
          updateMessages(currentMessages);
          break;
        }
        case 'Error': {
          onFinish('Stream error: ' + event.error);
          return;
        }
        case 'Finish': {
          onFinish();
          return;
        }
        case 'ModelChange': {
          break;
        }
        case 'UpdateConversation': {
          // WARNING: Since Message handler uses this local variable, we need to update it here to avoid the client clobbering it.
          // Longterm fix is to only send the agent the new messages, not the entire conversation.
          currentMessages = event.conversation;
          updateMessages(event.conversation);
          break;
        }
        case 'Notification': {
          updateNotifications(event as NotificationEvent);
          break;
        }
        case 'Ping':
          break;
      }
    }

    onFinish();
  } catch (error) {
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
  const abortControllerRef = useRef<AbortController | null>(null);

  useEffect(() => {
    if (session) {
      resultsCache.set(sessionId, { session, messages });
    }
  }, [sessionId, session, messages]);

  const updateMessages = useCallback((newMessages: Message[]) => {
    setMessages(newMessages);
    messagesRef.current = newMessages;
  }, []);

  const updateNotifications = useCallback((notification: NotificationEvent) => {
    setNotifications((prev) => [...prev, notification]);
  }, []);

  const onFinish = useCallback(
    async (error?: string): Promise<void> => {
      if (error) {
        setSessionLoadError(error);
      }

      const isNewSession = sessionId && sessionId.match(/^\d{8}_\d{6}$/);
      if (isNewSession) {
        console.log(
          'useChatStream: Message stream finished for new session, emitting message-stream-finished event'
        );
        window.dispatchEvent(new CustomEvent('message-stream-finished'));
      }

      setChatState(ChatState.Idle);
      onStreamFinish();
    },
    [onStreamFinish, sessionId]
  );

  // Load session on mount or sessionId change
  useEffect(() => {
    if (!sessionId) return;

    const cached = resultsCache.get(sessionId);
    if (cached) {
      setSession(cached.session);
      updateMessages(cached.messages);
      setTokenState({
        inputTokens: cached.session?.input_tokens ?? 0,
        outputTokens: cached.session?.output_tokens ?? 0,
        totalTokens: cached.session?.total_tokens ?? 0,
        accumulatedInputTokens: cached.session?.accumulated_input_tokens ?? 0,
        accumulatedOutputTokens: cached.session?.accumulated_output_tokens ?? 0,
        accumulatedTotalTokens: cached.session?.accumulated_total_tokens ?? 0,
      });
      setChatState(ChatState.Idle);
      return;
    }

    // Reset state when sessionId changes
    updateMessages([]);
    setSession(undefined);
    setSessionLoadError(undefined);
    setChatState(ChatState.LoadingConversation);

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

        const session = response.data;
        setSession(session);
        updateMessages(session?.conversation || []);
        setTokenState({
          inputTokens: session?.input_tokens ?? 0,
          outputTokens: session?.output_tokens ?? 0,
          totalTokens: session?.total_tokens ?? 0,
          accumulatedInputTokens: session?.accumulated_input_tokens ?? 0,
          accumulatedOutputTokens: session?.accumulated_output_tokens ?? 0,
          accumulatedTotalTokens: session?.accumulated_total_tokens ?? 0,
        });
        setChatState(ChatState.Idle);
        onSessionLoaded?.();
      } catch (error) {
        if (cancelled) return;

        setSessionLoadError(errorMessage(error));
        setChatState(ChatState.Idle);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [sessionId, updateMessages, onSessionLoaded]);

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

      const newMessage = hasNewMessage
        ? createUserMessage(userMessage)
        : messagesRef.current[messagesRef.current.length - 1];
      const currentMessages = hasNewMessage
        ? [...messagesRef.current, newMessage]
        : [...messagesRef.current];

      if (hasNewMessage) {
        updateMessages(currentMessages);
      }

      setChatState(ChatState.Streaming);
      setNotifications([]);
      abortControllerRef.current = new AbortController();

      try {
        const { stream } = await reply({
          body: {
            session_id: sessionId,
            user_message: newMessage,
          },
          throwOnError: true,
          signal: abortControllerRef.current.signal,
        });

        await streamFromResponse(
          stream,
          currentMessages,
          updateMessages,
          setTokenState,
          setChatState,
          updateNotifications,
          onFinish
        );
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
    [sessionId, session, chatState, updateMessages, updateNotifications, onFinish]
  );

  const submitElicitationResponse = useCallback(
    async (elicitationId: string, userData: Record<string, unknown>) => {
      if (!session || chatState === ChatState.LoadingConversation) {
        return;
      }

      const responseMessage = createElicitationResponseMessage(elicitationId, userData);
      const currentMessages = [...messagesRef.current, responseMessage];

      updateMessages(currentMessages);
      setChatState(ChatState.Streaming);
      setNotifications([]);
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

        await streamFromResponse(
          stream,
          currentMessages,
          updateMessages,
          setTokenState,
          setChatState,
          updateNotifications,
          onFinish
        );
      } catch (error) {
        if (error instanceof Error && error.name === 'AbortError') {
          // Silently handle abort
        } else {
          onFinish('Submit error: ' + errorMessage(error));
        }
      }
    },
    [sessionId, session, chatState, updateMessages, updateNotifications, onFinish]
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
        setSession({
          ...session,
          user_recipe_values,
        });
      } else {
        setSessionLoadError("can't call setRecipeParams without a session");
      }
    },
    [sessionId, session, setSessionLoadError]
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
    abortControllerRef.current?.abort();
    setChatState(ChatState.Idle);
  }, []);

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

  const cached = resultsCache.get(sessionId);
  const maybe_cached_messages = session ? messages : cached?.messages || [];
  const maybe_cached_session = session ?? cached?.session;

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
    messages: maybe_cached_messages,
    session: maybe_cached_session,
    chatState,
    handleSubmit,
    submitElicitationResponse,
    stopStreaming,
    setRecipeUserParams,
    tokenState,
    notifications: notificationsMap,
    onMessageUpdate,
  };
}
