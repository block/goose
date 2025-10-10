import { useState, useCallback, useRef } from 'react';
import { ChatState } from '../types/chatState';
import { Message } from '../api';
import { getApiUrl } from '../config';

const TextDecoder = globalThis.TextDecoder;

interface UseChatStreamProps {
  sessionId: string;
  messages: Message[];
  setMessages: (messages: Message[]) => void;
  onStreamFinish?: () => void;
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

export function useChatStream({
  sessionId,
  messages,
  setMessages,
  onStreamFinish,
}: UseChatStreamProps) {
  const [chatState, setChatState] = useState<ChatState>(ChatState.Idle);
  const abortControllerRef = useRef<AbortController | null>(null);

  // Use refs to track current state without triggering re-renders
  const messagesRef = useRef<Message[]>(messages);
  const sessionIdRef = useRef<string>(sessionId);

  // Keep refs in sync with props
  messagesRef.current = messages;
  sessionIdRef.current = sessionId;

  const handleSubmit = useCallback(
    async (userMessage: string) => {
      // Abort any existing stream
      if (abortControllerRef.current) {
        abortControllerRef.current.abort();
        abortControllerRef.current = null;
      }

      const newMessage: Message = {
        role: 'user',
        content: [{ type: 'text', text: userMessage }],
        created: Date.now(),
      };

      // Use the current messages from ref to avoid stale closure
      let currentMessages = [...messagesRef.current, newMessage];
      setMessages(currentMessages);
      setChatState(ChatState.Streaming);

      abortControllerRef.current = new AbortController();

      try {
        // TODO(Douwe): this side steps our API. heyapi does support streaming though which should make
        // this all nice & typed
        const response = await fetch(getApiUrl('/reply'), {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'X-Secret-Key': await window.electron.getSecretKey(),
          },
          body: JSON.stringify({
            session_id: sessionIdRef.current,
            messages: currentMessages,
          }),
          signal: abortControllerRef.current.signal,
        });

        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        if (!response.body) throw new Error('No response body');

        const reader = response.body.getReader();
        const decoder = new TextDecoder();

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

              if (event.message) {
                const msg = event.message as Message;
                currentMessages = pushMessage(currentMessages, msg);
                setMessages(currentMessages);
              }

              if (event.error) {
                console.error('Stream error:', event.error);
                setChatState(ChatState.Idle);
                return;
              }

              if (event.finish) {
                setChatState(ChatState.Idle);
                onStreamFinish?.();
                return;
              }
            } catch (e) {
              console.error('Failed to parse SSE:', e);
            }
          }
        }

        // Stream completed without explicit finish event
        setChatState(ChatState.Idle);
      } catch (error) {
        if (error instanceof Error && error.name !== 'AbortError') {
          console.error('Stream error:', error);
        }
        setChatState(ChatState.Idle);
      } finally {
        // Clean up abort controller
        if (abortControllerRef.current) {
          abortControllerRef.current = null;
        }
      }
    },
    [setMessages, onStreamFinish]
  );

  const stopStreaming = useCallback(() => {
    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
      abortControllerRef.current = null;
    }
    setChatState(ChatState.Idle);
  }, []);

  return {
    chatState,
    handleSubmit,
    stopStreaming,
  };
}
