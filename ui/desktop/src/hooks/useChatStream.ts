import { useCallback, useEffect, useRef, useState } from 'react';
import { ChatState } from '../types/chatState';
import { Conversation, Message, resumeAgent, Session } from '../api';
import { getApiUrl } from '../config';
import { createUserMessage } from '../types/message';

const resultsCache = new Map<string, { messages: Message[]; session: Session }>();

const TextDecoder = globalThis.TextDecoder;

type JsonValue = string | number | boolean | null | JsonValue[] | { [key: string]: JsonValue };

// TODO(Douwe): get these from the server:
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
  setMessages: (messages: Message[]) => void,
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
      if (done) break;

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
              setMessages(currentMessages);
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
              setMessages(event.conversation);
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
          onFinish('Failed to parse SSE:' + e);
        }
      }
    }
  } catch (error) {
    if (error instanceof Error && error.name !== 'AbortError') {
      onFinish('Stream error:' + error);
    }
  }
}

export function useChatStream({
  sessionId,
  onStreamFinish,
}: UseChatStreamProps): UseChatStreamReturn {
  const [messages, setMessages] = useState<Message[]>([]);
  const messagesRef = useRef<Message[]>([]);
  const [session, setSession] = useState<Session>();
  const [sessionLoadError, setSessionLoadError] = useState<string>();
  const [chatState, setChatState] = useState<ChatState>(ChatState.Idle);
  const abortControllerRef = useRef<AbortController | null>(null);

  const setMessagesAndCache = (messages: Message[], log: string) => {
    console.log(log);
    setMessages(messages);
    if (session) {
      resultsCache.set(session.id, { session, messages });
    }
  };

  const renderCountRef = useRef(0);
  renderCountRef.current += 1;
  console.log(`useChatStream render #${renderCountRef.current}, ${session?.id}`);

  useEffect(() => {
    messagesRef.current = messages;
  }, [messages]);

  const onFinish = useCallback(
    (error?: string): void => {
      setSessionLoadError(error);
      setChatState(ChatState.Idle);
      onStreamFinish();
    },
    [onStreamFinish]
  );

  useEffect(() => {
    if (!sessionId) return;

    setChatState(ChatState.Thinking);

    (async () => {
      try {
        console.log('Calling resumeAgent for', sessionId);
        const response = await resumeAgent({
          body: {
            session_id: sessionId,
            load_model_and_extensions: true,
          },
          throwOnError: true,
        });
        const session = response.data;
        setSession(session);
        setMessagesAndCache(session?.conversation || [], 'load-session');
        setChatState(ChatState.Idle);
      } catch (error) {
        setSessionLoadError(error instanceof Error ? error.message : String(error));
        setChatState(ChatState.Idle);
      }
    })();
  }, [sessionId, setMessagesAndCache]);

  const handleSubmit = useCallback(
    async (userMessage: string) => {
      const currentMessages = [...messagesRef.current, createUserMessage(userMessage)];
      setMessagesAndCache(currentMessages, 'user-entered');
      setChatState(ChatState.Streaming);

      abortControllerRef.current = new AbortController();

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
        (messages: Message[]) => {
          setMessagesAndCache(messages, 'streaming');
        },
        onFinish
      );
    },
    [sessionId, onFinish, setMessagesAndCache]
  );

  const stopStreaming = useCallback(() => {
    abortControllerRef.current?.abort();
    setChatState(ChatState.Idle);
  }, []);

  console.log('>> returning', messages.length, messages.length > 0, chatState);

  const cached = resultsCache.get(sessionId)

  return {
    sessionLoadError,
    session ? messages : cached?.messages,
    session ?? cached?.session,
    chatState,
    handleSubmit,
    stopStreaming,
  };
}
