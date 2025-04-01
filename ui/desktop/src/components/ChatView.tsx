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
import { DeepLinkModal } from './ui/DeepLinkModal';
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
  const [waitingForAgentResponse, setWaitingForAgentResponse] = useState(false);
  const [showShareableBotModal, setshowShareableBotModal] = useState(false);
  const [generatedBotConfig, setGeneratedBotConfig] = useState<any>(null);
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

  // Listen for make-agent-from-chat event
  useEffect(() => {
    const handleMakeAgent = async () => {
      window.electron.logInfo('Making agent from chat...');

      // Log all messages for now
      window.electron.logInfo('Current messages:');
      chat.messages.forEach((message, index) => {
        const role = isUserMessage(message) ? 'user' : 'assistant';
        const content = isUserMessage(message) ? message.text : getTextContent(message);
        window.electron.logInfo(`Message ${index} (${role}): ${content}`);
      });

      // Inject a question into the chat to generate instructions
      const instructionsPrompt =
        'Based on our conversation so far, could you create:\n' +
        "1. A concise set of instructions (1-2 paragraphs) that describe what you've been helping with. Pay special attention if any output styles or formats are requested (and make it clear), and note any non standard tools used or required.\n" +
        '2. A list of 3-5 example activities (as a few words each at most) that would be relevant to this topic\n\n' +
        "Format your response with clear headings for 'Instructions:' and 'Activities:' sections." +
        'For example, perhaps we have been discussing fruit and you might write:\n\n' +
        'Instructions:\nUsing web searches we find pictures of fruit, and always check what language to reply in.' +
        'Activities:\nShow pics of apples, say a random fruit, share a fruit fact';

      // Set waiting state to true before adding the prompt
      setWaitingForAgentResponse(true);

      // Add the prompt as a user message
      append(createUserMessage(instructionsPrompt));

      window.electron.logInfo('Injected instructions prompt into chat');
    };

    window.addEventListener('make-agent-from-chat', handleMakeAgent);

    return () => {
      window.removeEventListener('make-agent-from-chat', handleMakeAgent);
    };
  }, [append, chat.messages, setWaitingForAgentResponse]);

  // Listen for new messages and process agent response
  useEffect(() => {
    // Only process if we're waiting for an agent response
    if (!waitingForAgentResponse || messages.length === 0) {
      return;
    }

    // Get the last message
    const lastMessage = messages[messages.length - 1];

    // Check if it's an assistant message (response to our prompt)
    if (lastMessage.role === 'assistant') {
      // Extract the content
      const content = getTextContent(lastMessage);

      // Process the agent's response
      if (content) {
        window.electron.logInfo('Received agent response:');
        window.electron.logInfo(content);

        // Parse the response to extract instructions and activities
        const instructionsMatch = content.match(/Instructions:(.*?)(?=Activities:|$)/s);
        const activitiesMatch = content.match(/Activities:(.*?)$/s);

        const instructions = instructionsMatch ? instructionsMatch[1].trim() : '';
        const activitiesText = activitiesMatch ? activitiesMatch[1].trim() : '';

        // Parse activities into an array
        const activities = activitiesText
          .split(/\n+/)
          .map((line) => line.replace(/^[•\-*\d]+\.?\s*/, '').trim())
          .filter((activity) => activity.length > 0);

        // Create a bot config object
        const generatedConfig = {
          id: `bot-${Date.now()}`,
          name: 'Custom Bot',
          description: 'Bot created from chat',
          instructions: instructions,
          activities: activities,
        };

        window.electron.logInfo('Extracted bot config:');
        window.electron.logInfo(JSON.stringify(generatedConfig, null, 2));

        // Store the generated bot config
        setGeneratedBotConfig(generatedConfig);

        // Show the modal with the generated bot config
        setshowShareableBotModal(true);

        window.electron.logInfo('Generated bot config for agent creation');

        // Reset waiting state
        setWaitingForAgentResponse(false);
      }
    }
  }, [messages, waitingForAgentResponse, setshowShareableBotModal, setGeneratedBotConfig]);

  // Leaving these in for easy debugging of different message states

  // One message with a tool call and no text content
  // const messages = [{"role":"assistant","created":1742484893,"content":[{"type":"toolRequest","id":"call_udVcu3crnFdx2k5FzlAjk5dI","toolCall":{"status":"success","value":{"name":"developer__text_editor","arguments":{"command":"write","file_text":"Hello, this is a test file.\nLet's see if this works properly.","path":"/Users/alexhancock/Development/testfile.txt"}}}}]}];

  // One message with text content and tool calls
  // const messages = [{"role":"assistant","created":1742484388,"content":[{"type":"text","text":"Sure, let's break this down into two steps:\n\n1. **Write content to a `.txt` file.**\n2. **Read the content from the `.txt` file.**\n\nLet's start by writing some example content to a `.txt` file. I'll create a file named `example.txt` and write a sample sentence into it. Then I'll read the content back. \n\n### Sample Content\nWe'll write the following content into the `example.txt` file:\n\n```\nHello World! This is an example text file.\n```\n\nLet's proceed with this task."},{"type":"toolRequest","id":"call_CmvAsxMxiWVKZvONZvnz4QCE","toolCall":{"status":"success","value":{"name":"developer__text_editor","arguments":{"command":"write","file_text":"Hello World! This is an example text file.","path":"/Users/alexhancock/Development/example.txt"}}}}]}];

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
      <div className="relative flex items-center h-[36px] w-full">
        <MoreMenuLayout setView={setView} setIsGoosehintsModalOpen={setIsGoosehintsModalOpen} />
      </div>

      <Card className="flex flex-col flex-1 rounded-none h-[calc(100vh-95px)] w-full bg-bgApp mt-0 border-none relative">
        {messages.length === 0 ? (
          <Splash
            append={(text) => append(createUserMessage(text))}
            activities={botConfig?.activities || null}
          />
        ) : (
          <ScrollArea ref={scrollRef} className="flex-1" autoScroll>
            <SearchView scrollAreaRef={scrollRef}>
              {filteredMessages.map((message, index) => (
                <div key={message.id || index} className="mt-4 px-4">
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
            initialValue={_input}
          />
          <BottomMenu hasMessages={hasMessages} setView={setView} />
        </div>
      </Card>

      {showGame && <FlappyGoose onClose={() => setShowGame(false)} />}

      {/* Deep Link Modal */}
      {showShareableBotModal && generatedBotConfig && (
        <DeepLinkModal
          botConfig={generatedBotConfig}
          onClose={() => {
            setshowShareableBotModal(false);
            setGeneratedBotConfig(null);
          }}
          onOpen={() => {
            setshowShareableBotModal(false);
            setGeneratedBotConfig(null);
          }}
        />
      )}
    </div>
  );
}
