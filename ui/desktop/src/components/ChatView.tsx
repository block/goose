import React, { useEffect, useRef, useState } from 'react';
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
import { useMessageStream } from '../hooks/useMessageStream';
import { Message, createUserMessage, getTextContent } from '../types/message';

export interface ChatType {
  id: number;
  title: string;
  messages: Message[];
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

  const {
    messages,
    append,
    stop,
    isLoading,
    error,
    setMessages,
    input: _input,
    setInput: _setInput,
    handleInputChange: _handleInputChange,
    handleSubmit: _submitMessage,
  } = useMessageStream({
    api: getApiUrl('/reply'),
    initialMessages: chat?.messages || [],
    onFinish: async (message, _reason) => {
      window.electron.stopPowerSaveBlocker();

      // Disabled askAi calls to save costs
      // const messageText = getTextContent(message);
      // const fetchResponses = await askAi(messageText);
      // setMessageMetadata((prev) => ({ ...prev, [message.id || '']: fetchResponses }));

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
    onToolCall: (toolCall) => {
      // Handle tool calls if needed
      console.log('Tool call received:', toolCall);
      // Implement tool call handling logic here
    },
  });

  // Update chat messages when they change
  useEffect(() => {
    setChat((prevChat) => ({ ...prevChat, messages }));
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
      append(createUserMessage(content));
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

    // Handle stopping the message stream
    const lastMessage = messages[messages.length - 1];
    if (lastMessage && lastMessage.role === 'user') {
      // Remove the last user message if it's the most recent one
      if (messages.length > 1) {
        setMessages(messages.slice(0, -1));
      } else {
        setMessages([]);
      }
    }
    // Note: Tool call interruption handling would need to be implemented
    // differently with the new message format
  };

  // Filter out standalone tool response messages for rendering
  // They will be shown as part of the tool invocation in the assistant message
  const filteredMessages = messages.filter((message) => {
    // Keep all assistant messages and user messages that aren't just tool responses
    if (message.role === 'assistant') return true;

    // For user messages, check if they're only tool responses
    if (message.role === 'user') {
      const hasOnlyToolResponses = message.content.every((c) => c.type === 'toolResponse');
      const hasTextContent = message.content.some((c) => c.type === 'text');

      // Keep the message if it has text content or is not just tool responses
      return hasTextContent || !hasOnlyToolResponses;
    }

    return true;
  });

  return (
    <div className="flex flex-col w-full h-screen items-center justify-center">
      <div className="relative flex items-center h-[36px] w-full bg-bgSubtle border-b border-borderSubtle">
        <MoreMenu setView={setView} />
      </div>
      <Card className="flex flex-col flex-1 rounded-none h-[calc(100vh-95px)] w-full bg-bgApp mt-0 border-none relative">
        {messages.length === 0 ? (
          <Splash append={(text) => append(createUserMessage(text))} />
        ) : (
          <ScrollArea ref={scrollRef} className="flex-1 px-4" autoScroll>
            {filteredMessages.map((message, index) => (
              <div key={message.id || index} className="mt-[16px]">
                {message.role === 'user' ? (
                  <UserMessage message={message} />
                ) : (
                  <GooseMessage
                    message={message}
                    messages={messages}
                    metadata={messageMetadata[message.id || '']}
                    append={(text) => append(createUserMessage(text))}
                  />
                )}
              </div>
            ))}
            {error && (
              <div className="flex flex-col items-center justify-center p-4">
                <div className="text-red-700 dark:text-red-300 bg-red-400/50 p-3 rounded-lg mb-2">
                  {error.message || 'Honk! Goose experienced an error while responding'}
                </div>
                <div
                  className="px-3 py-2 mt-2 text-center whitespace-nowrap cursor-pointer text-textStandard border border-borderSubtle hover:bg-bgSubtle rounded-full inline-block transition-all duration-150"
                  onClick={async () => {
                    // Find the last user message
                    const lastUserMessage = messages.reduceRight(
                      (found, m) => found || (m.role === 'user' ? m : null),
                      null as Message | null
                    );
                    if (lastUserMessage) {
                      append(lastUserMessage);
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
          <Input handleSubmit={handleSubmit} isLoading={isLoading} onStop={onStopGoose} />
          <BottomMenu hasMessages={hasMessages} setView={setView} />
        </div>
      </Card>

      {showGame && <FlappyGoose onClose={() => setShowGame(false)} />}
    </div>
  );
}
