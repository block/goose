import React, { useEffect, useRef, useState } from 'react';
import { Message, useChat } from '../ai-sdk-fork/useChat';
import { getApiUrl } from '../config';
import BottomMenu from './BottomMenu';
import FlappyGoose from './FlappyGoose';
import GooseMessage from './GooseMessage';
import Input from './Input';
import { type View } from '../App';
import LoadingGoose from './LoadingGoose';
import MoreMenu from './MoreMenu';
import { Card } from './ui/card';
import { ScrollArea, ScrollAreaHandle } from './ui/scroll-area';
import UserMessage from './UserMessage';
import { askAi } from '../utils/askAI';
import Splash from './Splash';
import 'react-toastify/dist/ReactToastify.css';

export interface ChatType {
  id: number;
  title: string;
  messages: Array<{
    id: string;
    role: 'function' | 'system' | 'user' | 'assistant' | 'data' | 'tool';
    content: string;
  }>;
}

export default function ChatView({ setView }: { setView: (view: View) => void }) {
  const [chat, setChat] = useState<ChatType>(() => {
    return {
      id: 1,
      title: 'Chat 1',
      messages: [],
    };
  });
  const [messageMetadata, setMessageMetadata] = useState<Record<string, string[]>>({});
  const [hasMessages, setHasMessages] = useState(false);
  const [lastInteractionTime, setLastInteractionTime] = useState<number>(Date.now());
  const [showGame, setShowGame] = useState(false);
  const scrollRef = useRef<ScrollAreaHandle>(null);

  const { messages, append, stop, isLoading, error, setMessages } = useChat({
    api: getApiUrl('/reply'),
    initialMessages: chat?.messages || [],
    onFinish: async (message, _) => {
      window.electron.stopPowerSaveBlocker();

      const fetchResponses = await askAi(message.content);
      setMessageMetadata((prev) => ({ ...prev, [message.id]: fetchResponses }));

      const timeSinceLastInteraction = Date.now() - lastInteractionTime;
      window.electron.logInfo('last interaction:' + lastInteractionTime);
      if (timeSinceLastInteraction > 60000) {
        // 60000ms = 1 minute
        window.electron.showNotification({
          title: 'Goose finished the task.',
          body: 'Click here to expand.',
        });
      }
    },
  });

  // Update chat messages when they change
  useEffect(() => {
    setChat({ ...chat, messages });
  }, [messages]);

  useEffect(() => {
    if (messages.length > 0) {
      setHasMessages(true);
    }
  }, [messages]);

  // Handle submit
  const handleSubmit = (e: React.FormEvent) => {
    window.electron.startPowerSaveBlocker();
    const customEvent = e as CustomEvent;
    const content = customEvent.detail?.value || '';
    if (content.trim()) {
      setLastInteractionTime(Date.now());
      append({
        role: 'user',
        content,
      });
      if (scrollRef.current?.scrollToBottom) {
        scrollRef.current.scrollToBottom();
      }
    }
  };

  if (error) {
    console.log('Error:', error);
  }

  const onStopGoose = () => {
    stop();
    setLastInteractionTime(Date.now());
    window.electron.stopPowerSaveBlocker();

    const lastMessage: Message = messages[messages.length - 1];
    if (lastMessage.role === 'user' && lastMessage.toolInvocations === undefined) {
      // Remove the last user message.
      if (messages.length > 1) {
        setMessages(messages.slice(0, -1));
      } else {
        setMessages([]);
      }
    } else if (lastMessage.role === 'assistant' && lastMessage.toolInvocations !== undefined) {
      // Add messaging about interrupted ongoing tool invocations
      const newLastMessage: Message = {
        ...lastMessage,
        toolInvocations: lastMessage.toolInvocations.map((invocation) => {
          if (invocation.state !== 'result') {
            return {
              ...invocation,
              result: [
                {
                  audience: ['user'],
                  text: 'Interrupted.\n',
                  type: 'text',
                },
                {
                  audience: ['assistant'],
                  text: 'Interrupted by the user to make a correction.\n',
                  type: 'text',
                },
              ],
              state: 'result',
            };
          } else {
            return invocation;
          }
        }),
      };

      const updatedMessages = [...messages.slice(0, -1), newLastMessage];
      setMessages(updatedMessages);
    }
  };

  return (
    <div className="flex flex-col w-full h-screen items-center justify-center">
      <div className="relative flex items-center h-[36px] w-full bg-bgSubtle border-b border-borderSubtle">
        <MoreMenu setView={setView} />
      </div>
      <Card className="flex flex-col flex-1 rounded-none h-[calc(100vh-95px)] w-full bg-bgApp mt-0 border-none relative">
        {messages.length === 0 ? (
          <Splash append={append} />
        ) : (
          <ScrollArea ref={scrollRef} className="flex-1 px-4" autoScroll>
            {messages.map((message) => (
              <div key={message.id} className="mt-[16px]">
                {message.role === 'user' ? (
                  <UserMessage message={message} />
                ) : (
                  <GooseMessage
                    message={message}
                    messages={messages}
                    metadata={messageMetadata[message.id]}
                    append={append}
                  />
                )}
              </div>
            ))}
            {error && (
              <div className="flex flex-col items-center justify-center p-4">
                <div className="text-red-700 dark:text-red-300 bg-red-400/50 p-3 rounded-lg mb-2">
                  {error.message || 'Honk! Goose experienced an error while responding'}
                  {error.status && <span className="ml-2">(Status: {error.status})</span>}
                </div>
                <div
                  className="px-3 py-2 mt-2 text-center whitespace-nowrap cursor-pointer text-textStandard border border-borderSubtle hover:bg-bgSubtle rounded-full inline-block transition-all duration-150"
                  onClick={async () => {
                    const lastUserMessage = messages.reduceRight(
                      (found, m) => found || (m.role === 'user' ? m : null),
                      null
                    );
                    if (lastUserMessage) {
                      append({
                        role: 'user',
                        content: lastUserMessage.content,
                      });
                    }
                  }}
                >
                  Retry Last Message
                </div>
              </div>
            )}
            <div className="block h-16" />
          </ScrollArea>
        )}

        <div className="relative">
          {isLoading && <LoadingGoose />}
          <Input
            handleSubmit={handleSubmit}
            disabled={isLoading}
            isLoading={isLoading}
            onStop={onStopGoose}
          />
          <BottomMenu hasMessages={hasMessages} setView={setView} />
        </div>
      </Card>

      {showGame && <FlappyGoose onClose={() => setShowGame(false)} />}
    </div>
  );
}
