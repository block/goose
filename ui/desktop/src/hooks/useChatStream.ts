import { useCallback, useEffect, useRef, useState } from 'react';
import { ChatState } from '../types/chatState';
import { Message, resumeAgent, Session } from '../api';
import { getApiUrl } from '../config';
import { createUserMessage, generateMessageId } from '../types/message';

const TextDecoder = globalThis.TextDecoder;

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
  onMessage: (messages: Message[]) => void,
  onStateChange: (state: ChatState) => void,
  onSession: (session: Session) => void,
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
          const event = JSON.parse(data);

          if (event.type === 'SessionSnapshot') {
            const session = event.session;
            console.log('>>got session', session.description, session.conversation?.length);
            onMessage(session.conversation || []);
            session.conversation = [];
            onSession(session);
          }

          if (event.type === 'Message' || event.message) {
            const msg = (event.message || event.message) as Message;
            currentMessages = pushMessage(currentMessages, msg);
            onMessage(currentMessages);
            onStateChange(ChatState.Streaming);
          }

          if (event.type === 'Error' || event.error) {
            onFinish('Stream error: ' + event.error);
            return;
          }

          if (event.type === 'Finish' || event.finish) {
            onFinish();
            return;
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

  // Load session using resumeAgent instead of subscribe
  useEffect(() => {
    if (!sessionId) return;

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
        const session = response.data;
        setSession(session);
        const messages = (session?.conversation || []).map((m) =>
          m.id ? m : { ...m, id: generateMessageId() }
        );
        setMessages(messages);
        setChatState(ChatState.Idle);
      } catch (error) {
        setSessionLoadError(error instanceof Error ? error.message : String(error));
        setChatState(ChatState.Idle);
      }
    })();
  }, [sessionId]);

  const handleSubmit = useCallback(
    async (userMessage: string) => {
      const currentMessages = [...messagesRef.current, createUserMessage(userMessage)];
      setMessages(currentMessages);
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
        setMessages,
        setChatState,
        setSession,
        onFinish
      );
    },
    [sessionId, onFinish]
  );

  const stopStreaming = useCallback(() => {
    abortControllerRef.current?.abort();
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
