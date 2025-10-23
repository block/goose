import { useCallback, useEffect, useRef, useState } from 'react';
import { ChatState } from '../types/chatState';
import { Conversation, Message, resumeAgent, Session } from '../api';
import { getApiUrl } from '../config';
import { createUserMessage } from '../types/message';

const TextDecoder = globalThis.TextDecoder;
const resultsCache = new Map<string, { messages: Message[]; session: Session }>();

type JsonValue = string | number | boolean | null | JsonValue[] | { [key: string]: JsonValue };

interface TokenState {
  input_tokens?: number | null;
  output_tokens?: number | null;
  total_tokens?: number | null;
  accumulated_input_tokens?: number | null;
  accumulated_output_tokens?: number | null;
  accumulated_total_tokens?: number | null;
}

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
  | { type: 'Message'; message: Message; token_state?: TokenState | null }
  | { type: 'Error'; error: string }
  | { type: 'Ping' }
  | { type: 'Finish'; reason: string }
  | { type: 'ModelChange'; model: string; mode: string }
  | { type: 'UpdateConversation'; conversation: Conversation }
  | NotificationEvent;

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
  stopStreaming: () => void;
  sessionLoadError?: string;
  tokenState?: TokenState;
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
  updateTokenState: (tokenState?: TokenState) => void,
  onFinish: (error?: string) => void
): Promise<void> {
  try {
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    if (!response.body) throw new Error('No response body');

    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let currentMessages = initialMessages;

    while (true) {
      const { done, value } = await reader.read();
      if (done) {
        break;
      }

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
              const msg = event.message;
              currentMessages = pushMessage(currentMessages, msg);

              // Update token state if present
              if (event.token_state) {
                updateTokenState(event.token_state);
              }

              updateMessages(currentMessages);
              break;
            }
            case 'Error': {
              console.error('Stream event error:', event.error);
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
              currentMessages = event.conversation;
              updateMessages(event.conversation);
              break;
            }
            case 'Notification': {
              break;
            }
            case 'Ping': {
              break;
            }
            default: {
              console.warn('Unhandled event type:', event['type']);
              break;
            }
          }
        } catch (e) {
          console.error('SSE parse failed:', e);
          onFinish('Failed to parse SSE:' + e);
        }
      }
    }
  } catch (error) {
    if (error instanceof Error && error.name !== 'AbortError') {
      console.error('Stream read error:', error);
      onFinish('Stream error:' + error);
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
  const [tokenState, setTokenState] = useState<TokenState>();
  const abortControllerRef = useRef<AbortController | null>(null);

  useEffect(() => {
    if (session) {
      resultsCache.set(sessionId, { session, messages });
    }
  }, [sessionId, session, messages]);

  const setMessagesAndLog = useCallback((newMessages: Message[]) => {
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
    setMessagesAndLog([]);
    setSession(undefined);
    setSessionLoadError(undefined);
    setChatState(ChatState.Thinking);

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
        if (cancelled) return;

        const session = response.data;
        setSession(session);
        setMessagesAndLog(session?.conversation || []);
        setChatState(ChatState.Idle);
      } catch (error) {
        if (cancelled) return;

        console.error('Session load failed:', error);
        setSessionLoadError(error instanceof Error ? error.message : String(error));
        setChatState(ChatState.Idle);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [sessionId, setMessagesAndLog]);

  const handleSubmit = useCallback(
    async (userMessage: string) => {
      const currentMessages = [...messagesRef.current, createUserMessage(userMessage)];
      setMessagesAndLog(currentMessages);
      setChatState(ChatState.Streaming);

      abortControllerRef.current = new AbortController();

      try {
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

        await streamFromResponse(
          response,
          currentMessages,
          (messages: Message[]) => setMessagesAndLog(messages),
          setTokenState,
          onFinish
        );
      } catch (error) {
        // AbortError is expected when user stops streaming
        if (error instanceof Error && error.name === 'AbortError') {
          // Stream was aborted by user
        } else {
          // Unexpected error during fetch setup (streamFromResponse handles its own errors)
          console.error('Submit failed:', error);
          onFinish('Submit error: ' + (error instanceof Error ? error.message : String(error)));
        }
      }
    },
    [sessionId, setMessagesAndLog, onFinish]
  );

  useEffect(() => {
    if (initialMessage && session && messages.length === 0 && chatState === ChatState.Idle) {
      handleSubmit(initialMessage);
    }
  }, [initialMessage, session, messages.length, chatState, handleSubmit]);

  const stopStreaming = useCallback(() => {
    abortControllerRef.current?.abort();
    setChatState(ChatState.Idle);
  }, []);

  const cached = resultsCache.get(sessionId);
  const maybe_cached_messages = session ? messages : cached?.messages || [];
  const maybe_cached_session = session ?? cached?.session;

  return {
    sessionLoadError,
    messages: maybe_cached_messages,
    session: maybe_cached_session,
    chatState,
    handleSubmit,
    stopStreaming,
    tokenState,
  };
}
