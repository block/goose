/// <reference lib="dom" />
import React, { useEffect, useRef, useState, useMemo, useCallback } from 'react';
import { getApiUrl } from '../config';
import FlappyGoose from './FlappyGoose';
import GooseMessage from './GooseMessage';
import Input from './Input';
import { type View, ViewOptions } from '../App';
import LoadingGoose from './LoadingGoose';
import MoreMenuLayout from './more_menu/MoreMenuLayout';
import { Card } from './ui/card';
import { ScrollArea, ScrollAreaHandle } from './ui/scroll-area';
import UserMessage from './UserMessage';
import Splash from './Splash';
import { SearchView } from './conversation/SearchView';
import { createRecipe } from '../recipe';
import { AgentHeader } from './AgentHeader';
import LayingEggLoader from './LayingEggLoader';
import { fetchSessionDetails, generateSessionId } from '../sessions';
import 'react-toastify/dist/ReactToastify.css';
import { useMessageStream } from '../hooks/useMessageStream';
import { SessionSummaryModal } from './context_management/SessionSummaryModal';
import { Recipe } from '../recipe';
import {
  ChatContextManagerProvider,
  useChatContextManager,
} from './context_management/ChatContextManager';
import { ContextHandler } from './context_management/ContextHandler';
import { LocalMessageStorage } from '../utils/localMessageStorage';
import {
  Message,
  createUserMessage,
  ToolCall,
  ToolCallResult,
  ToolRequestMessageContent,
  ToolResponseMessageContent,
  ToolConfirmationRequestMessageContent,
  getTextContent,
} from '../types/message';
import { useDropzone } from 'react-dropzone';
import {} from /* Removed Attach */ './icons';
import type { BotConfig } from '../botConfig';

export interface ChatType {
  id: string;
  title: string;
  messageHistoryIndex: number;
  messages: Message[];
}

// Define a type for files dropped, intersecting the standard File type
// with an object containing the optional non-standard 'path' property added by Electron.
// eslint-disable-next-line no-undef
type DroppedFile = File & { path?: string };

// Define isUserMessage helper function (integrated from upstream/main)
const isUserMessage = (message: Message): boolean => {
  if (message.role === 'assistant') {
    return false;
  }
  if (message.content.every((c) => c.type === 'toolConfirmationRequest')) {
    return false;
  }
<<<<<<< HEAD
  // Assuming EnableExtensionRequestMessageContent is not used in HEAD, keep this commented or remove
  // if (message.content.every((c) => c.type === 'enableExtensionRequest')) {
  //   return false;
  // }
=======
>>>>>>> main
  return true;
};

export default function ChatView({
  chat,
  setChat,
  setView,
  setIsGoosehintsModalOpen,
}: {
  chat: ChatType;
  setChat: (chat: ChatType) => void;
  setView: (view: View, viewOptions?: Record<string, unknown>) => void;
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
}) {
<<<<<<< HEAD
  const [_messageMetadata, _setMessageMetadata] = useState<Record<string, string[]>>({});
  const [lastInteractionTime, setLastInteractionTime] = useState<number>(Date.now());
  const [showGame, setShowGame] = useState(false);
  const [waitingForAgentResponse, setWaitingForAgentResponse] = useState(false);
  const [generatedBotConfig, setGeneratedBotConfig] = useState<BotConfig | null>(null);
=======
  return (
    <ChatContextManagerProvider>
      <ChatContent
        chat={chat}
        setChat={setChat}
        setView={setView}
        setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
      />
    </ChatContextManagerProvider>
  );
}

function ChatContent({
  chat,
  setChat,
  setView,
  setIsGoosehintsModalOpen,
}: {
  chat: ChatType;
  setChat: (chat: ChatType) => void;
  setView: (view: View, viewOptions?: ViewOptions) => void;
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
}) {
  const [hasMessages, setHasMessages] = useState(false);
  const [lastInteractionTime, setLastInteractionTime] = useState<number>(Date.now());
  const [showGame, setShowGame] = useState(false);
  const [isGeneratingRecipe, setIsGeneratingRecipe] = useState(false);
  const [sessionTokenCount, setSessionTokenCount] = useState<number>(0);
  const [ancestorMessages, setAncestorMessages] = useState<Message[]>([]);
  const [droppedFiles, setDroppedFiles] = useState<string[]>([]);

>>>>>>> main
  const scrollRef = useRef<ScrollAreaHandle>(null);
  const [_showDeepLinkModal, _setShowDeepLinkModal] = useState<boolean>(false);
  const [_deepLinkUrl, _setDeepLinkUrl] = useState<string>('');
  const [attachedImages, setAttachedImages] = useState<string[]>([]);
  const [value, setValue] = useState('');

  const {
    summaryContent,
    summarizedThread,
    isSummaryModalOpen,
    resetMessagesWithSummary,
    closeSummaryModal,
    updateSummary,
    hasContextHandlerContent,
    getContextHandlerType,
  } = useChatContextManager();

  useEffect(() => {
    // Log all messages when the component first mounts
    window.electron.logInfo(
      'Initial messages when resuming session: ' + JSON.stringify(chat.messages, null, 2)
    );
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []); // Empty dependency array means this runs once on mount;

  // Get recipeConfig directly from appConfig
  // const recipeConfig = window.appConfig.get('recipeConfig') as Recipe | null;

  // Store message in global history when it's added
  const storeMessageInHistory = useCallback((message: Message) => {
    if (isUserMessage(message)) {
      const text = getTextContent(message);
      if (text) {
        LocalMessageStorage.addMessage(text);
      }
    }
  }, []);

  const {
    messages,
    append: originalAppend,
    stop,
    isLoading,
    error,
    setMessages,
    input: _input,
    setInput: _setInput,
    handleInputChange: _handleInputChange,
    handleSubmit: _submitMessage,
    updateMessageStreamBody,
  } = useMessageStream({
    api: getApiUrl('/reply'),
    initialMessages: chat.messages,
    body: { session_id: chat.id, session_working_dir: window.appConfig.get('GOOSE_WORKING_DIR') },
    onFinish: async (_message, _reason) => {
      window.electron.stopPowerSaveBlocker();

      setTimeout(() => {
        if (scrollRef.current?.scrollToBottom) {
          scrollRef.current.scrollToBottom();
        }
      }, 300);

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

  // Wrap append to store messages in global history
  const append = useCallback(
    (messageOrString: Message | string) => {
      const message =
        typeof messageOrString === 'string' ? createUserMessage(messageOrString) : messageOrString;
      storeMessageInHistory(message);
      return originalAppend(message);
    },
    [originalAppend, storeMessageInHistory]
  );

  // for CLE events -- create a new session id for the next set of messages
  useEffect(() => {
    // If we're in a continuation session, update the chat ID
    if (summarizedThread.length > 0) {
      const newSessionId = generateSessionId();

      // Update the session ID in the chat object
      setChat({
        ...chat,
        id: newSessionId!,
        title: `Continued from ${chat.id}`,
        messageHistoryIndex: summarizedThread.length,
      });

      // Update the body used by useMessageStream to send future messages to the new session
      if (summarizedThread.length > 0 && updateMessageStreamBody) {
        updateMessageStreamBody({
          session_id: newSessionId,
          session_working_dir: window.appConfig.get('GOOSE_WORKING_DIR'),
        });
      }
    }

    // only update if summarizedThread length changes from 0 -> 1+
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    // eslint-disable-next-line react-hooks/exhaustive-deps
    summarizedThread.length > 0,
  ]);

  // Listen for make-agent-from-chat event
  useEffect(() => {
    const handleMakeAgent = async () => {
      window.electron.logInfo('Making agent from chat (using HEAD logic)...');
      // Assuming setIsGeneratingRecipe is not needed for bot creation logic
      // setIsGeneratingRecipe(true);

      // Log all messages for now
      window.electron.logInfo('Current messages:');
      chat.messages.forEach((message, index) => {
        const role = isUserMessage(message) ? 'user' : 'assistant';
        const content = getTextContent(message);
        window.electron.logInfo(`Message ${index} (${role}): ${content}`);
      });

      // Construct the prompt for the agent
      const agentPrompt = `Based on the following conversation, generate instructions and suggested activities for a specialized bot:

${chat.messages
  .map((message) => {
    const role = isUserMessage(message) ? 'User' : 'Assistant';
    const content = getTextContent(message);
    return `${role}: ${content}`;
  })
  .join('\n\n')}

Provide the output in the following format:
Instructions: [Detailed instructions for the bot based on the conversation]
Activities: [Bulleted list of suggested user activities based on the conversation]`;

      window.electron.logInfo('Generated prompt for agent creation:');
      window.electron.logInfo(agentPrompt);

      // Send the prompt as a new user message
      append(createUserMessage(agentPrompt));

      // Set waiting state to process the response
      setWaitingForAgentResponse(true);
      window.electron.logInfo('Sent prompt to generate bot config');
      
      try {
        window.electron.logInfo('Opening recipe editor window');
      } catch (error) {
        window.electron.logInfo('Failed to create recipe:');
        const errorMessage = error instanceof Error ? error.message : String(error);
        window.electron.logInfo(errorMessage);
      } finally {
        setIsGeneratingRecipe(false);
      }
    };

    window.addEventListener('make-agent-from-chat', handleMakeAgent);

    return () => {
      window.removeEventListener('make-agent-from-chat', handleMakeAgent);
    };
  }, [append, chat.messages, setWaitingForAgentResponse, chat, messages, setIsGeneratingRecipe]);

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
        const instructionsMatch = content.match(/Instructions:([\s\S]*?)(?=Activities:|$)/);
        const activitiesMatch = content.match(/Activities:([\s\S]*?)$/);

        const instructions = instructionsMatch ? instructionsMatch[1].trim() : '';
        const activitiesText = activitiesMatch ? activitiesMatch[1].trim() : '';

        // Parse activities into an array
        const activities = activitiesText
          .split(/\n+/)
          .map((line) => line.replace(/^[•\-*\d]+\.?\s*/, '').trim())
          .filter((activity) => activity.length > 0);

        // Create a bot config object
        const generatedConfig: BotConfig = {
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
        // Assuming the modal display is handled elsewhere or this state/setter was truly unused
        // setshowShareableBotModal(true);

        window.electron.logInfo('Generated bot config for agent creation');

        // Reset waiting state
        setWaitingForAgentResponse(false);
      }
    }
  }, [messages, waitingForAgentResponse, setGeneratedBotConfig]);

  // Update parent component's chat state when messages change
  useEffect(() => {
    // Avoid function update form if not supported by setChat type
    // Pass the latest chat metadata along with the updated messages
    // Check if messages reference has actually changed before setting state
    // Note: This relies on parent passing a stable 'chat' reference if messages haven't changed, which might not be guaranteed.
    // A potentially safer approach might involve memoizing parts of the chat object in the parent.
    setChat({
      ...chat, // Spread existing chat props (id, title, etc.)
      messages: messages, // Update with the latest messages array
    });
  }, [messages, chat.id, chat.title, chat.messageHistoryIndex, setChat, chat]);

  // Updated Dropzone Logic
  const onDrop = useCallback(
    (acceptedFiles: DroppedFile[]) => {
      // Use the DroppedFile type to access path safely
      acceptedFiles.forEach((file) => {
        // Use inferred DroppedFile type
        console.log('Dropped file object:', file);
        if (file.type.startsWith('image/')) {
          const reader = new FileReader();
          reader.onabort = () => console.log('file reading was aborted');
          reader.onerror = () => console.log('file reading has failed');
          reader.onload = () => {
            const base64Image = reader.result as string;
            setAttachedImages((prevImages) => [...prevImages, base64Image]);
          };
          reader.readAsDataURL(file);
        } else {
          // Access path directly from the DroppedFile type
          const filePath = file.path;
          const fileName = file.name; // Get filename as fallback
          const cwd = window.appConfig.get('GOOSE_WORKING_DIR');
          let finalPath: string | null = null;

          if (filePath && typeof filePath === 'string') {
            // Check if path looks absolute
            if (filePath.startsWith('/') || filePath.match(/^[a-zA-Z]:\\/)) {
              finalPath = filePath; // Use absolute path directly
            } else {
              // Path is relative, clean it and join with CWD
              const cleanedRelativePath = filePath.startsWith('./')
                ? filePath.substring(2)
                : filePath;
              finalPath = `${cwd}/${cleanedRelativePath}`;
            }
          } else if (fileName) {
            // Fallback: If path is missing, use CWD + filename
            console.warn('File path missing, falling back to CWD + filename');
            finalPath = `${cwd}/${fileName}`;
          }

          if (finalPath) {
            // Normalize slashes and append
            finalPath = finalPath.replace(/\\/g, '/');
            setValue((prevValue) => `${prevValue}${prevValue ? ' ' : ''}${finalPath}`.trimStart());
          } else {
            console.error('Could not get path or name for non-image file');
          }
        }
      });
    },
    [setValue, setAttachedImages]
  );

  const { getRootProps, getInputProps, isDragActive } = useDropzone({
    onDrop,
    noClick: true,
    noKeyboard: true,
  });

  const removeAttachedImage = (indexToRemove: number) => {
    setAttachedImages((prevImages) => prevImages.filter((_, index) => index !== indexToRemove));
  };

  // Updated handleSubmit to use local value state
  const handleSubmit = (
    e?: React.FormEvent | CustomEvent<{ value?: string /* This detail value is no longer used */ }>
  ) => {
    e?.preventDefault(); // Prevent default form submission if triggered by form event
    window.electron.startPowerSaveBlocker();
<<<<<<< HEAD
    setLastInteractionTime(Date.now());

    // Use the value state from ChatView
    const textToSend = value.trim();

    if (textToSend || attachedImages.length > 0) {
      const userMessage = createUserMessage(textToSend, attachedImages);
      append(userMessage);

      // Clear state after submit
      setValue(''); // Clear text input state
      setAttachedImages([]);

      if (scrollRef.current?.scrollToBottom) {
        scrollRef.current.scrollToBottom();
=======
    const customEvent = e as unknown as CustomEvent;
    const content = customEvent.detail?.value || '';

    if (content.trim()) {
      setLastInteractionTime(Date.now());

      if (summarizedThread.length > 0) {
        // move current `messages` to `ancestorMessages` and `messages` to `summarizedThread`
        resetMessagesWithSummary(
          messages,
          setMessages,
          ancestorMessages,
          setAncestorMessages,
          summaryContent
        );

        // update the chat with new sessionId

        // now call the llm
        setTimeout(() => {
          append(createUserMessage(content));
          if (scrollRef.current?.scrollToBottom) {
            scrollRef.current.scrollToBottom();
          }
        }, 150);
      } else {
        // Normal flow (existing code)
        append(createUserMessage(content));
        if (scrollRef.current?.scrollToBottom) {
          scrollRef.current.scrollToBottom();
        }
>>>>>>> main
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
<<<<<<< HEAD
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

        // Wrap toolResponse in a Message object for append
        const responseMessage: Message = {
          role: 'user', // Or 'system' if more appropriate for interruption feedback
          created: Date.now(),
          content: [toolResponse], // Put the single ToolResponseMessageContent in an array
        };
        append(responseMessage); // Append the full Message object
=======

      if (toolRequests.length !== 0) {
        // This means we were interrupted during a tool request
        // Create tool responses for all interrupted tool requests

        let responseMessage: Message = {
          display: true,
          sendToLLM: true,
          role: 'user',
          created: Date.now(),
          content: [],
        };

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
>>>>>>> main
      }
    }
  };

  // Filter out standalone tool response messages for rendering
  // They will be shown as part of the tool invocation in the assistant message
  const filteredMessages = [...ancestorMessages, ...messages].filter((message) => {
    // Only filter out when display is explicitly false
    if (message.display === false) return false;

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

  const commandHistory = useMemo(() => {
    const filteredMessages = messages || [];
    return filteredMessages
      .filter((m) => m.role === 'user' && getTextContent(m)?.trim())
      .map((m) => getTextContent(m))
      .reverse();
  }, [messages]);

  const hasMessages = messages.length > 0;

  // Fetch session metadata to get token count
  useEffect(() => {
    const fetchSessionTokens = async () => {
      try {
        const sessionDetails = await fetchSessionDetails(chat.id);
        setSessionTokenCount(sessionDetails.metadata.total_tokens);
      } catch (err) {
        console.error('Error fetching session token count:', err);
      }
    };
    if (chat.id) {
      fetchSessionTokens();
    }
  }, [chat.id, messages]);

  const handleDrop = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    const files = e.dataTransfer.files;
    if (files.length > 0) {
      const paths: string[] = [];
      for (let i = 0; i < files.length; i++) {
        paths.push(window.electron.getPathForFile(files[i]));
      }
      setDroppedFiles(paths);
    }
  };

  const handleDragOver = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
  };

  return (
    <div className="flex flex-col w-full h-screen items-center justify-center">
      {/* Loader when generating recipe */}
<<<<<<< HEAD
      {/* {isGeneratingRecipe && <LayingEggLoader />} */}
      <div className="relative flex items-center h-[36px] w-full">
        <MoreMenuLayout setView={setView} setIsGoosehintsModalOpen={setIsGoosehintsModalOpen} />
      </div>

      <Card {...getRootProps()} className="flex flex-col h-full w-full overflow-hidden relative">
        {/* Prevent dropzone activation when clicking inside */}
        <div onClick={(e) => e.stopPropagation()} className="flex flex-col flex-1 h-full">
          {/* Main Content Area */}
          <div className="flex flex-col flex-1 rounded-none h-[calc(100vh-95px)] w-full bg-bgApp mt-0 border-none relative">
            {messages.length === 0 && !isLoading ? (
              <Splash
                append={(text) => append(createUserMessage(text))}
                activities={generatedBotConfig?.activities || null}
              />
            ) : (
              <ScrollArea ref={scrollRef} className="flex-1 overflow-y-auto p-4">
                <SearchView>
                  {filteredMessages.map((message, index) => (
                    <div key={message.id || index} className="mt-4 px-4">
                      {isUserMessage(message) ? (
                        <UserMessage message={message} />
=======
      {isGeneratingRecipe && <LayingEggLoader />}
      <MoreMenuLayout
        hasMessages={hasMessages}
        setView={setView}
        setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
      />

      <Card
        className="flex flex-col flex-1 rounded-none h-[calc(100vh-95px)] w-full bg-bgApp mt-0 border-none relative"
        onDrop={handleDrop}
        onDragOver={handleDragOver}
      >
        {recipeConfig?.title && messages.length > 0 && (
          <AgentHeader
            title={recipeConfig.title}
            profileInfo={
              recipeConfig.profile
                ? `${recipeConfig.profile} - ${recipeConfig.mcps || 12} MCPs`
                : undefined
            }
            onChangeProfile={() => {
              // Handle profile change
              console.log('Change profile clicked');
            }}
          />
        )}
        {messages.length === 0 ? (
          <Splash
            append={append}
            activities={Array.isArray(recipeConfig?.activities) ? recipeConfig.activities : null}
            title={recipeConfig?.title}
          />
        ) : (
          <ScrollArea ref={scrollRef} className="flex-1" autoScroll>
            <SearchView>
              {filteredMessages.map((message, index) => (
                <div
                  key={message.id || index}
                  className="mt-4 px-4"
                  data-testid="message-container"
                >
                  {isUserMessage(message) ? (
                    <>
                      {hasContextHandlerContent(message) ? (
                        <ContextHandler
                          messages={messages}
                          messageId={message.id ?? message.created.toString()}
                          chatId={chat.id}
                          workingDir={window.appConfig.get('GOOSE_WORKING_DIR') as string}
                          contextType={getContextHandlerType(message)}
                        />
                      ) : (
                        <UserMessage message={message} />
                      )}
                    </>
                  ) : (
                    <>
                      {/* Only render GooseMessage if it's not a message invoking some context management */}
                      {hasContextHandlerContent(message) ? (
                        <ContextHandler
                          messages={messages}
                          messageId={message.id ?? message.created.toString()}
                          chatId={chat.id}
                          workingDir={window.appConfig.get('GOOSE_WORKING_DIR') as string}
                          contextType={getContextHandlerType(message)}
                        />
>>>>>>> main
                      ) : (
                        <GooseMessage
                          messageHistoryIndex={chat?.messageHistoryIndex}
                          message={message}
                          messages={messages}
<<<<<<< HEAD
                          metadata={_messageMetadata[message.id || '']}
                          append={(text) => append(createUserMessage(text))}
=======
                          append={append}
>>>>>>> main
                          appendMessage={(newMessage) => {
                            const updatedMessages = [...messages, newMessage];
                            setMessages(updatedMessages);
                          }}
                        />
                      )}
<<<<<<< HEAD
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
              </ScrollArea>
            )}

            <div className="relative">
              {isLoading && <LoadingGoose />}
              <Input
                value={value}
                setValue={setValue}
                handleSubmit={handleSubmit}
                isLoading={isLoading}
                onStop={onStopGoose}
                commandHistory={commandHistory}
                isDragActive={isDragActive}
                getInputProps={getInputProps}
                attachedImages={attachedImages}
                setAttachedImages={setAttachedImages}
                removeAttachedImage={removeAttachedImage}
              />
              <BottomMenu hasMessages={hasMessages} setView={setView} />
            </div>
          </div>
=======
                    </>
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
            <div className="block h-8" />
          </ScrollArea>
        )}

        <div className="relative p-4 pt-0 z-10 animate-[fadein_400ms_ease-in_forwards]">
          {isLoading && <LoadingGoose />}
          <ChatInput
            handleSubmit={handleSubmit}
            isLoading={isLoading}
            onStop={onStopGoose}
            commandHistory={commandHistory}
            initialValue={_input}
            setView={setView}
            hasMessages={hasMessages}
            numTokens={sessionTokenCount}
            droppedFiles={droppedFiles}
            messages={messages}
            setMessages={setMessages}
          />
>>>>>>> main
        </div>
      </Card>

      {showGame && <FlappyGoose onClose={() => setShowGame(false)} />}

      <SessionSummaryModal
        isOpen={isSummaryModalOpen}
        onClose={closeSummaryModal}
        onSave={(editedContent) => {
          updateSummary(editedContent);
          closeSummaryModal();
        }}
        summaryContent={summaryContent}
      />
    </div>
  );
}
