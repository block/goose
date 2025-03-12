import React, { useEffect, useRef, useState, useMemo } from 'react';
import { getApiUrl } from '../config';
import { generateSessionId } from '../sessions';
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
import {
  Message,
  createUserMessage,
  ToolCall,
  ToolCallResult,
  ToolRequestMessageContent,
  ToolResponse,
  ToolResponseMessageContent,
  ToolConfirmationRequestMessageContent,
  getTextContent,
  createAssistantMessage,
} from '../types/message';

export interface ChatType {
  id: string;
  title: string;
  // messages up to this index are presumed to be "history" from a resumed session, this is used to track older tool confirmation requests
  // anything before this index should not render any buttons, but anything after should
  messageHistoryIndex: number;
  messages: Message[];
}

export default function ChatView({
  chat,
  setChat,
  setView,
  setIsGoosehintsModalOpen,
}: {
  chat: ChatType;
  setChat: (chat: ChatType) => void;
  setView: (view: View, viewOptions?: Record<any, any>) => void;
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
}) {
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
    initialMessages: chat.messages,
    body: { session_id: chat.id, session_working_dir: window.appConfig.get('GOOSE_WORKING_DIR') },
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

  // Update chat messages when they change and save to sessionStorage
  useEffect(() => {
    setChat((prevChat) => {
      const updatedChat = { ...prevChat, messages };
      return updatedChat;
    });
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

    // check if the last user message has any tool response(s)
    const isToolResponse = lastMessage.content.some(
      (content): content is ToolResponseMessageContent => content.type == 'toolResponse'
    );

    // isUserMessage also checks if the message is a toolConfirmationRequest
    // check if the last message is a real user's message
    if (lastMessage && isUserMessage(lastMessage) && !isToolResponse) {
      // Remove the last user message if it's the most recent one
      if (messages.length > 1) {
        setMessages(messages.slice(0, -1));
      } else {
        setMessages([]);
      }
      // Interruption occured after a tool has completed, but no assistant reply
      // handle his if we want to popup a message too the user
      // } else if (lastMessage && isUserMessage(lastMessage) && isToolResponse) {
    } else if (!isUserMessage(lastMessage)) {
      // the last message was an assistant message
      // check if we have any tool requests or tool confirmation requests
      const toolRequests: [string, ToolCallResult<ToolCall>][] = lastMessage.content
        .filter(
          (content): content is ToolRequestMessageContent | ToolConfirmationRequestMessageContent =>
            content.type === 'toolRequest' || content.type === 'toolConfirmationRequest'
        )
        .map((content) => {
          if (content.type === 'toolRequest') {
            return [content.id, content.toolCall];
          } else {
            // extract tool call from confirmation
            const toolCall: ToolCallResult<ToolCall> = {
              status: 'success',
              value: {
                name: content.toolName,
                arguments: content.arguments,
              },
            };
            return [content.id, toolCall];
          }
        });

      if (toolRequests.length !== 0) {
        // This means we were interrupted during a tool request
        // Create tool responses for all interrupted tool requests

        let responseMessage: Message = {
          role: 'user',
          created: Date.now(),
          content: [],
        };

        // get the last tool's name or just "tool"
        const lastToolName = toolRequests.at(-1)?.[1].value?.name ?? 'tool';
        const notification = 'Interrupted by the user to make a correction';

        // generate a response saying it was interrupted for each tool request
        for (const [reqId, _] of toolRequests) {
          const toolResponse: ToolResponseMessageContent = {
            type: 'toolResponse',
            id: reqId,
            toolResult: {
              status: 'error',
              error: notification,
            },
          };

          responseMessage.content.push(toolResponse);
        }

        // Use an immutable update to add the response message to the messages array
        setMessages([...messages, responseMessage]);
      }
    }
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
      const hasToolConfirmation = message.content.every(
        (c) => c.type === 'toolConfirmationRequest'
      );

      // Keep the message if it has text content or tool confirmation or is not just tool responses
      return hasTextContent || !hasOnlyToolResponses || hasToolConfirmation;
    }

    return true;
  });

  const isUserMessage = (message: Message) => {
    if (message.role === 'assistant') {
      return false;
    }

    if (message.content.every((c) => c.type === 'toolConfirmationRequest')) {
      return false;
    }
    return true;
  };

  const commandHistory = useMemo(() => {
    return filteredMessages
      .reduce<string[]>((history, message) => {
        if (isUserMessage(message)) {
          const text = message.content.find((c) => c.type === 'text')?.text?.trim();
          if (text) {
            history.push(text);
          }
        }
        return history;
      }, [])
      .reverse();
  }, [filteredMessages, isUserMessage]);

  return (
    <div className="flex flex-col w-full h-screen items-center justify-center">
      <div className="relative flex items-center h-[36px] w-full bg-bgSubtle border-b border-borderSubtle">
        <MoreMenu setView={setView} setIsGoosehintsModalOpen={setIsGoosehintsModalOpen} />
      </div>
      <Card className="flex flex-col flex-1 rounded-none h-[calc(100vh-95px)] w-full bg-bgApp mt-0 border-none relative">
        {messages.length === 0 ? (
          <Splash append={(text) => append(createUserMessage(text))} />
        ) : (
          <ScrollArea ref={scrollRef} className="flex-1 px-4" autoScroll>
            {filteredMessages.map((message, index) => (
              <div key={message.id || index} className="mt-[16px]">
                {isUserMessage(message) ? (
                  <UserMessage message={message} />
                ) : (
                  <GooseMessage
                    messageHistoryIndex={chat?.messageHistoryIndex}
                    message={message}
                    messages={messages}
                    metadata={messageMetadata[message.id || '']}
                    append={(text) => append(createUserMessage(text))}
                    appendMessage={(newMessage) => {
                      const updatedMessages = [...messages, newMessage];
                      setMessages(updatedMessages);
                    }}
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
          <Input
            handleSubmit={handleSubmit}
            isLoading={isLoading}
            onStop={onStopGoose}
            commandHistory={commandHistory}
          />
          <BottomMenu hasMessages={hasMessages} setView={setView} />
        </div>
      </Card>

      {showGame && <FlappyGoose onClose={() => setShowGame(false)} />}
    </div>
  );
}
