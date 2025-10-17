import { useCallback, useEffect, useRef, useState } from 'react';
import { ChatState } from '../types/chatState';
import { Conversation, Message, resumeAgent, Session } from '../api';
import { getApiUrl } from '../config';
import { createUserMessage } from '../types/message';

const TextDecoder = globalThis.TextDecoder;

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

type JsonValue = string | number | boolean | null | JsonValue[] | { [key: string]: JsonValue };

interface NotificationEvent {
  type: 'Notification';
  request_id: string;
  message: {
    method: string;
    params: {
      [key: string]: JsonValue;
    };
  };
}

type MessageEvent =
  | { type: 'Message'; message: Message }
  | { type: 'Error'; error: string }
  | { type: 'Ping' }
  | { type: 'Finish'; reason: string }
  | { type: 'ModelChange'; model: string; mode: string }
  | { type: 'UpdateConversation'; conversation: Conversation }
  | NotificationEvent;

interface UseChatStreamProps {
  sessionId: string;
  onStreamFinish: () => void;
}

interface UseChatStreamReturn {
  session?: Session;
  messages: Message[];
  chatState: ChatState;
  handleSubmit: (userMessage: string) => Promise<void>;
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
  response: Response,
  initialMessages: Message[],
  updateMessages: (messages: Message[]) => void,
  onFinish: (error?: string) => void
): Promise<void> {
  let chunkCount = 0;
  let messageEventCount = 0;

  try {
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    if (!response.body) throw new Error('No response body');

    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let currentMessages = initialMessages;

    log.stream('reading-chunks');

    while (true) {
      const { done, value } = await reader.read();
      if (done) {
        log.stream('chunks-complete', {
          totalChunks: chunkCount,
          messageEvents: messageEventCount,
        });
        break;
      }

      chunkCount++;
      const chunk = decoder.decode(value);
      const lines = chunk.split('\n');

      for (const line of lines) {
        if (!line.startsWith('data: ')) continue;

        const data = line.slice(6);
        if (data === '[DONE]') continue;

        try {
          const event = JSON.parse(data) as MessageEvent;

          switch (event.type) {
            case 'Message': {
              messageEventCount++;
              const msg = event.message;
              currentMessages = pushMessage(currentMessages, msg);

              // Only log every 10th message event to avoid spam
              if (messageEventCount % 10 === 0) {
                log.stream('message-chunk', {
                  eventCount: messageEventCount,
                  messageCount: currentMessages.length,
                });
              }

              // This calls the wrapped setMessagesAndLog with 'streaming' context
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
              // This calls the wrapped setMessagesAndLog with 'streaming' context
              updateMessages(event.conversation);
              break;
            }
            case 'Notification': {
              // Don't log notifications, too noisy
              break;
            }
            case 'Ping': {
              // Don't log pings
              break;
            }
            default: {
              console.warn('Unhandled event type:', event['type']);
              break;
            }
          }
        } catch (e) {
          log.error('SSE parse failed', e);
          onFinish('Failed to parse SSE:' + e);
        }
      }
    }
  } catch (error) {
    if (error instanceof Error && error.name !== 'AbortError') {
      log.error('stream read error', error);
      onFinish('Stream error:' + error);
    }
  }
}

export function useChatStream({
  sessionId,
  onStreamFinish,
}: UseChatStreamProps): UseChatStreamReturn {
  const [messages, _setMessages] = useState<Message[]>([]);
  const messagesRef = useRef<Message[]>([]);
  const [session, setSession] = useState<Session>();
  const [sessionLoadError, setSessionLoadError] = useState<string>();
  const [chatState, setChatState] = useState<ChatState>(ChatState.Idle);
  const abortControllerRef = useRef<AbortController | null>(null);
  const sessionIdRef = useRef<string>(sessionId);

  const renderCountRef = useRef(0);
  renderCountRef.current += 1;
  console.log(`useChatStream render #${renderCountRef.current}, ${session?.id}`);

  const setMessagesAndLog = useCallback((newMessages: Message[], logContext: string) => {
    log.messages(logContext, newMessages.length, {
      lastMessageRole: newMessages[newMessages.length - 1]?.role,
      lastMessageId: newMessages[newMessages.length - 1]?.id?.slice(0, 8),
    });
    _setMessages(newMessages);
    messagesRef.current = newMessages;
  }, []);

  useEffect(() => {
    if (sessionIdRef.current !== sessionId) {
      log.session('changed', sessionId, {
        from: sessionIdRef.current.slice(0, 8),
        to: sessionId.slice(0, 8),
      });
      sessionIdRef.current = sessionId;
      setMessagesAndLog([], 'session-reset');
      setSession(undefined);
      setSessionLoadError(undefined);
      setChatState(ChatState.Idle);
    }
  }, [sessionId, setMessagesAndLog]);

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

    let cancelled = false;

    log.session('loading', sessionId);
    log.state(ChatState.Thinking, { reason: 'session load start' });
    setChatState(ChatState.Thinking);

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

        const sessionData = response.data;
        log.session('loaded', sessionId, {
          messageCount: sessionData?.conversation?.length || 0,
          description: sessionData?.description,
        });

        setSession(sessionData);
        setMessagesAndLog(sessionData?.conversation || [], 'load-session');

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

        const response = await fetch(getApiUrl('/reply'), {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'X-Secret-Key': await window.electron.getSecretKey(),
          },
          body: JSON.stringify({
            session_id: sessionId,
            messages: currentMessages,
          }),
          signal: abortControllerRef.current.signal,
        });

        log.stream('response-received', {
          status: response.status,
          ok: response.ok,
        });

        await streamFromResponse(
          response,
          currentMessages,
          (messages: Message[]) => setMessagesAndLog(messages, 'streaming'),
          onFinish
        );

        log.stream('stream-complete');
      } catch (error) {
        if (error instanceof Error && error.name !== 'AbortError') {
          log.error('submit failed', error);
          onFinish('Submit error: ' + error.message);
        } else if (error instanceof Error && error.name === 'AbortError') {
          log.stream('stream-aborted');
        }
      }
    },
    [sessionId, setMessagesAndLog, onFinish]
  );

  const stopStreaming = useCallback(() => {
    log.stream('stop-requested');
    abortControllerRef.current?.abort();
    log.state(ChatState.Idle, { reason: 'user stopped streaming' });
    setChatState(ChatState.Idle);
  }, []);

  return {
    sessionLoadError,
    messages,
    session,
    chatState,
    handleSubmit,
    stopStreaming,
  };
}
