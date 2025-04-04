import React, { useEffect, useRef, useState, useMemo } from 'react';
import { getApiUrl } from '../config';
import BottomMenu from './BottomMenu';
import FlappyGoose from './FlappyGoose';
import GooseMessage from './GooseMessage';
import Input from './Input';
import { type View } from '../App';
import LoadingGoose from './LoadingGoose';
import MoreMenuLayout from './more_menu/MoreMenuLayout';
import { Card } from './ui/card';
import { ScrollArea, ScrollAreaHandle } from './ui/scroll-area';
import UserMessage from './UserMessage';
import Splash from './Splash';
import { SearchView } from './conversation/SearchView';
import { createGooseling, loadGooseling } from '../api-client';
import { configureGooselingExtensions } from '../utils/gooselingExtensions';
import 'react-toastify/dist/ReactToastify.css';
import { useMessageStream } from '../hooks/useMessageStream';
import { BotConfig } from '../botConfig';
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
  const [isGeneratingGooseling, setIsGeneratingGooseling] = useState(false);
  const scrollRef = useRef<ScrollAreaHandle>(null);

  // Get botConfig directly from appConfig
  const botConfig = window.appConfig.get('botConfig') as BotConfig | null;

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
    },
  });

  // Listen for make-agent-from-chat event
  useEffect(() => {
    const handleMakeAgent = async () => {
      window.electron.logInfo('Making gooseling from chat...');
      setIsGeneratingGooseling(true);

      try {
        // Create gooseling directly from chat messages
        const createGooselingRequest = {
          messages: messages,
          title: 'Custom Gooseling', // This will be updated by the server based on the conversation
          description: 'Created from chat session',
        };

        const response = await createGooseling(createGooselingRequest);

        window.electron.logInfo('Created gooseling:');
        window.electron.logInfo(JSON.stringify(response.gooseling, null, 2));

        // Create a new window for the gooseling editor
        window.electron.createChatWindow(
          undefined, // query
          undefined, // dir
          undefined, // version
          undefined, // resumeSessionId
          response.gooseling, // gooseling config
          'gooselingEditor' // view type
        );

        window.electron.logInfo('Opening gooseling editor window');
      } catch (error) {
        window.electron.logInfo('Failed to create gooseling:');
        window.electron.logInfo(error.message);
        // TODO: Show error to user
      } finally {
        setIsGeneratingGooseling(false);
      }
    };

    window.addEventListener('make-agent-from-chat', handleMakeAgent);

    return () => {
      window.removeEventListener('make-agent-from-chat', handleMakeAgent);
    };
  }, [messages]);

  // Handle loading gooselings
  useEffect(() => {
    const handleLoadGooseling = async (_, gooselingConfig) => {
      console.log('handleLoadGooseling called with config:', gooselingConfig);
      try {
        window.electron.logInfo('Loading gooseling with config:');
        window.electron.logInfo(JSON.stringify(gooselingConfig, null, 2));

        // Get config values
        // const provider = window.appConfig.get('GOOSE_PROVIDER');
        // const model = window.appConfig.get('GOOSE_MODEL');
        // console.log('Using provider/model:', { provider, model });

        // if (!provider) {
        //   throw new Error('No provider configured - please configure a provider first');
        // }

        // Configure extensions first with notifications enabled (not silent)
        if (gooselingConfig.extensions) {
          console.log('Found extensions to configure:', gooselingConfig.extensions);
          window.electron.logInfo('Configuring extensions:');
          window.electron.logInfo(JSON.stringify(gooselingConfig.extensions));
          await configureGooselingExtensions(gooselingConfig.extensions);
        }

        // Ensure activities are properly formatted
        const formattedConfig = {
          ...gooselingConfig,
          activities: Array.isArray(gooselingConfig.activities) ? gooselingConfig.activities : [],
        };

        // Update the chat state to start fresh
        setChat((prevChat) => ({
          ...prevChat,
          messageHistoryIndex: -1,
          messages: [],
        }));

        // Update the app config
        window.electron.emit('update-config', { botConfig: formattedConfig });

        window.electron.logInfo('Successfully loaded gooseling with configuration:');
        window.electron.logInfo(JSON.stringify(gooselingConfig, null, 2));
      } catch (err) {
        console.error('Failed to load gooseling:', err);
        window.electron.logInfo('Failed to load gooseling:', err);
        window.electron.logInfo('Error details:', {
          message: err.message,
          stack: err.stack,
        });
      }
    };

    // Listen for load-gooseling events
    window.electron.on('load-gooseling', handleLoadGooseling);

    // Initialize gooseling from botConfig if present
    const initialGooselingConfig = window.appConfig.get('botConfig');
    console.log('Checking for initial botConfig:', initialGooselingConfig);
    if (initialGooselingConfig) {
      handleLoadGooseling(null, initialGooselingConfig);
    }

    return () => {
      window.electron.off('load-gooseling', handleLoadGooseling);
    };
  }, [setChat]);

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
      // Get the text content from the last message before removing it
      const textContent = lastMessage.content.find((c) => c.type === 'text')?.text || '';

      // Set the text back to the input field
      _setInput(textContent);

      // Remove the last user message if it's the most recent one
      if (messages.length > 1) {
        setMessages(messages.slice(0, -1));
      } else {
        setMessages([]);
      }
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
      {/* Overlay when generating gooseling */}
      {isGeneratingGooseling && (
        <div className="fixed inset-0 bg-black/20 dark:bg-white/20 backdrop-blur-[1px] z-50 pointer-events-auto cursor-not-allowed" />
      )}
      <div className="relative flex items-center h-[36px] w-full">
        <MoreMenuLayout setView={setView} setIsGoosehintsModalOpen={setIsGoosehintsModalOpen} />
      </div>

      <Card className="flex flex-col flex-1 rounded-none h-[calc(100vh-95px)] w-full bg-bgApp mt-0 border-none relative">
        {messages.length === 0 ? (
          <Splash
            append={(text) => append(createUserMessage(text))}
            activities={Array.isArray(botConfig?.activities) ? botConfig.activities : null}
          />
        ) : (
          <ScrollArea ref={scrollRef} className="flex-1 px-4" autoScroll>
            <SearchView className="mt-[16px]" scrollAreaRef={scrollRef}>
              {filteredMessages.map((message, index) => (
                <div key={message.id || index} className="mt-[16px] message-content">
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
            </SearchView>
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
            value={_input}
            onValueChange={_setInput}
          />
          <BottomMenu hasMessages={hasMessages} setView={setView} />
        </div>
      </Card>

      {showGame && <FlappyGoose onClose={() => setShowGame(false)} />}
    </div>
  );
}
