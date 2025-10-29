import { useCallback, useEffect, useRef, useState } from 'react';
import { ChatState } from '../types/chatState';

import {
  Message,
  MessageEvent,
  reply,
  resumeAgent,
  Session,
  updateFromSession,
  updateSessionUserRecipeValues,
} from '../api';

import { createUserMessage, getCompactingMessage, getThinkingMessage } from '../types/message';

const resultsCache = new Map<string, { messages: Message[]; session: Session }>();

// Debug logging - set to false in production
const DEBUG_CHAT_STREAM = true;

const log = {
  session: (action: string, sessionId: string, details?: Record<string, unknown>) => {
    if (!DEBUG_CHAT_STREAM) return;
    console.log(`[useChatStream:session] ${action}`, {
      sessionId: sessionId.slice(0, 8),
      ...details,
    });
  },
  messages: (action: string, count: number, details?: Record<string, unknown>) => {
    if (!DEBUG_CHAT_STREAM) return;
    console.log(`[useChatStream:messages] ${action}`, {
      count,
      ...details,
    });
  },
  stream: (action: string, details?: Record<string, unknown>) => {
    if (!DEBUG_CHAT_STREAM) return;
    console.log(`[useChatStream:stream] ${action}`, details);
  },
  state: (newState: ChatState, details?: Record<string, unknown>) => {
    if (!DEBUG_CHAT_STREAM) return;
    console.log(`[useChatStream:state] → ${newState}`, details);
  },
  error: (context: string, error: unknown) => {
    console.error(`[useChatStream:error] ${context}`, error);
  },
};

interface UseChatStreamProps {
  sessionId: string;
  onStreamFinish: () => void;
  initialMessage?: string;
}

interface UseChatStreamReturn {
  session?: Session;
  messages: Message[];
  chatState: ChatState;
  handleSubmit: (userMessage: string) => Promise<void>;
  setRecipeUserParams: (values: Record<string, string>) => Promise<void>;
  stopStreaming: () => void;
  sessionLoadError?: string;
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
  updateChatState: (state: ChatState) => void,
  onFinish: (error?: string) => void
): Promise<void> {
  let messageEventCount = 0;
  let currentMessages = initialMessages;

  try {
    log.stream('reading-events');

    for await (const event of stream) {
      switch (event.type) {
        case 'Message': {
          messageEventCount++;
          const msg = event.message;
          currentMessages = pushMessage(currentMessages, msg);

          if (getCompactingMessage(msg)) {
            log.state(ChatState.Compacting, { reason: 'compacting notification' });
            updateChatState(ChatState.Compacting);
          } else if (getThinkingMessage(msg)) {
            log.state(ChatState.Thinking, { reason: 'thinking notification' });
            updateChatState(ChatState.Thinking);
          }

          if (messageEventCount % 10 === 0) {
            log.stream('message-chunk', {
              eventCount: messageEventCount,
              messageCount: currentMessages.length,
            });
          }

          updateMessages(currentMessages);
          break;
        }
        case 'Error': {
          log.error('stream event error', event.error);
          onFinish('Stream error: ' + event.error);
          return;
        }
        case 'Finish': {
          log.stream('finish-event', { reason: event.reason });
          onFinish();
          return;
        }
        case 'ModelChange': {
          log.stream('model-change', {
            model: event.model,
            mode: event.mode,
          });
          break;
        }
        case 'UpdateConversation': {
          log.messages('conversation-update', event.conversation.length);
          currentMessages = event.conversation;
          updateMessages(event.conversation);
          break;
        }
        case 'Notification':
        case 'Ping':
          break;
      }
    }

    log.stream('events-complete', { messageEvents: messageEventCount });
    onFinish();
  } catch (error) {
    if (error instanceof Error && error.name !== 'AbortError') {
      log.error('stream read error', error);
      onFinish('Stream error: ' + error);
    }
  }
}

export function useChatStream({
  sessionId,
  onStreamFinish,
  initialMessage,
}: UseChatStreamProps): UseChatStreamReturn {
  const [messages, setMessages] = useState<Message[]>([]);
  const messagesRef = useRef<Message[]>([]);
  const [session, setSession] = useState<Session>();
  const [sessionLoadError, setSessionLoadError] = useState<string>();
  const [chatState, setChatState] = useState<ChatState>(ChatState.Idle);
  const abortControllerRef = useRef<AbortController | null>(null);

  useEffect(() => {
    if (session) {
      resultsCache.set(sessionId, { session, messages });
    }
  }, [sessionId, session, messages]);

  const renderCountRef = useRef(0);
  renderCountRef.current += 1;
  console.log(`useChatStream render #${renderCountRef.current}, ${session?.id}`);

  const setMessagesAndLog = useCallback((newMessages: Message[], logContext: string) => {
    log.messages(logContext, newMessages.length, {
      lastMessageRole: newMessages[newMessages.length - 1]?.role,
      lastMessageId: newMessages[newMessages.length - 1]?.id?.slice(0, 8),
    });
    setMessages(newMessages);
    messagesRef.current = newMessages;
  }, []);

  const onFinish = useCallback(
    (error?: string): void => {
      if (error) {
        setSessionLoadError(error);
      }
      setChatState(ChatState.Idle);
      onStreamFinish();
    },
    [onStreamFinish]
  );

  // Load session on mount or sessionId change
  useEffect(() => {
    if (!sessionId) return;

    // Reset state when sessionId changes
    log.session('loading', sessionId);
    setMessagesAndLog([], 'session-reset');
    setSession(undefined);
    setSessionLoadError(undefined);
    setChatState(ChatState.LoadingConversation);

    let cancelled = false;

    log.state(ChatState.LoadingConversation, { reason: 'session load start' });

    (async () => {
      try {
        const response = await resumeAgent({
          body: {
            session_id: sessionId,
            load_model_and_extensions: true,
          },
          throwOnError: true,
        });
        if (cancelled) return;

        const session = response.data;
        log.session('loaded', sessionId, {
          messageCount: session?.conversation?.length || 0,
          name: session?.name,
        });

        setSession(session);
        setMessagesAndLog(session?.conversation || [], 'load-session');

        log.state(ChatState.Idle, { reason: 'session load complete' });
        setChatState(ChatState.Idle);
      } catch (error) {
        if (cancelled) return;

        log.error('session load failed', error);
        setSessionLoadError(error instanceof Error ? error.message : String(error));

        log.state(ChatState.Idle, { reason: 'session load error' });
        setChatState(ChatState.Idle);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [sessionId, setMessagesAndLog]);

  const handleSubmit = useCallback(
    async (userMessage: string) => {
      log.messages('user-submit', messagesRef.current.length + 1, {
        userMessageLength: userMessage.length,
      });

      const currentMessages = [...messagesRef.current, createUserMessage(userMessage)];
      setMessagesAndLog(currentMessages, 'user-entered');

      log.state(ChatState.Streaming, { reason: 'user submit' });
      setChatState(ChatState.Streaming);

      abortControllerRef.current = new AbortController();

      try {
        log.stream('request-start', { sessionId: sessionId.slice(0, 8) });

        const { stream } = await reply({
          body: {
            session_id: sessionId,
            messages: currentMessages,
          },
          throwOnError: true,
          signal: abortControllerRef.current.signal,
        });

        log.stream('stream-started');

        await streamFromResponse(
          stream,
          currentMessages,
          (messages: Message[]) => setMessagesAndLog(messages, 'streaming'),
          setChatState,
          onFinish
        );

        log.stream('stream-complete');
      } catch (error) {
        // AbortError is expected when user stops streaming
        if (error instanceof Error && error.name === 'AbortError') {
          log.stream('stream-aborted');
        } else {
          // Unexpected error during fetch setup (streamFromResponse handles its own errors)
          log.error('submit failed', error);
          onFinish('Submit error: ' + (error instanceof Error ? error.message : String(error)));
        }
      }
    },
    [sessionId, setMessagesAndLog, onFinish]
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

  useEffect(() => {
    if (initialMessage && session && messages.length === 0 && chatState === ChatState.Idle) {
      log.messages('auto-submit-initial', 0, { initialMessage: initialMessage.slice(0, 50) });
      handleSubmit(initialMessage);
    }
  }, [initialMessage, session, messages.length, chatState, handleSubmit]);

  const stopStreaming = useCallback(() => {
    log.stream('stop-requested');
    abortControllerRef.current?.abort();
    log.state(ChatState.Idle, { reason: 'user stopped streaming' });
    setChatState(ChatState.Idle);
  }, []);

  const cached = resultsCache.get(sessionId);
  const maybe_cached_messages = session ? messages : cached?.messages || [];
  const maybe_cached_session = session ?? cached?.session;

  console.log('>> returning', sessionId, Date.now(), maybe_cached_messages, chatState);

  return {
    sessionLoadError,
    messages: maybe_cached_messages,
    session: maybe_cached_session,
    chatState,
    handleSubmit,
    stopStreaming,
    setRecipeUserParams,
  };
}
