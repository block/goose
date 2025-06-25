import React, {
  useEffect,
  useRef,
  useState,
  useMemo,
  useCallback,
  createContext,
  useContext,
} from 'react';
import { getApiUrl } from '../config';
import FlappyGoose from './FlappyGoose';
import GooseMessage from './GooseMessage';
import ChatInput from './ChatInput';
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
  TextContent,
  SessionFile,
} from '../types/message';

// Constants for image handling
const MAX_IMAGES_PER_MESSAGE = 5;
const MAX_IMAGE_SIZE_MB = 5;

// Context for sharing current model info
const CurrentModelContext = createContext<{ model: string; mode: string } | null>(null);
export const useCurrentModelInfo = () => useContext(CurrentModelContext);

export interface ChatType {
  id: string;
  title: string;
  messageHistoryIndex: number;
  messages: Message[];
}

// Helper function to determine if a message is a user message
const isUserMessage = (message: Message): boolean => {
  if (message.role === 'assistant') {
    return false;
  }
  if (message.content.every((c) => c.type === 'toolConfirmationRequest')) {
    return false;
  }
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
  setView: (view: View, viewOptions?: ViewOptions) => void;
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
}) {
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
  const [sessionFiles, setSessionFiles] = useState<SessionFile[]>([]);

  const scrollRef = useRef<ScrollAreaHandle>(null);

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

  // Get recipeConfig directly from appConfig
  const recipeConfig = window.appConfig.get('recipeConfig') as Recipe | null;

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
    notifications,
    currentModelInfo,
  } = useMessageStream({
    api: getApiUrl('/reply'),
    initialMessages: chat.messages,
    body: {
      session_id: chat.id,
      session_working_dir: window.appConfig.get('GOOSE_WORKING_DIR'),
      ...(recipeConfig?.scheduledJobId && { scheduled_job_id: recipeConfig.scheduledJobId }),
    },
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
      window.electron.logInfo('Making recipe from chat...');
      setIsGeneratingRecipe(true);

      try {
        // Create recipe directly from chat messages
        const createRecipeRequest = {
          messages: messages,
          title: '',
          description: '',
        };

        const response = await createRecipe(createRecipeRequest);

        if (response.error) {
          throw new Error(`Failed to create recipe: ${response.error}`);
        }

        window.electron.logInfo('Created recipe:');
        window.electron.logInfo(JSON.stringify(response.recipe, null, 2));

        // First, verify the recipe data
        if (!response.recipe) {
          throw new Error('No recipe data received');
        }

        // Create a new window for the recipe editor
        console.log('Opening recipe editor with config:', response.recipe);
        const recipeConfig = {
          id: response.recipe.title || 'untitled',
          name: response.recipe.title || 'Untitled Recipe', // Does not exist on recipe type
          title: response.recipe.title || 'Untitled Recipe',
          description: response.recipe.description || '',
          instructions: response.recipe.instructions || '',
          activities: response.recipe.activities || [],
          prompt: response.recipe.prompt || '',
        };
        window.electron.createChatWindow(
          undefined, // query
          undefined, // dir
          undefined, // version
          undefined, // resumeSessionId
          recipeConfig, // recipe config
          'recipeEditor' // view type
        );

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
  }, [messages]);

  // Update chat messages when they change and save to sessionStorage
  useEffect(() => {
    // @ts-expect-error - TypeScript being overly strict about the return type
    setChat((prevChat: ChatType) => ({ ...prevChat, messages }));
  }, [messages, setChat]);

  useEffect(() => {
    if (messages.length > 0) {
      setHasMessages(true);
    }
  }, [messages]);

  // Auto-send the prompt for scheduled executions
  useEffect(() => {
    if (
      recipeConfig?.isScheduledExecution &&
      recipeConfig?.prompt &&
      messages.length === 0 &&
      !isLoading
    ) {
      console.log('Auto-sending prompt for scheduled execution:', recipeConfig.prompt);

      // Create and send the user message
      const userMessage = createUserMessage(recipeConfig.prompt);
      setLastInteractionTime(Date.now());
      window.electron.startPowerSaveBlocker();
      append(userMessage);

      // Scroll to bottom after sending
      setTimeout(() => {
        if (scrollRef.current?.scrollToBottom) {
          scrollRef.current.scrollToBottom();
        }
      }, 100);
    }
  }, [
    recipeConfig?.isScheduledExecution,
    recipeConfig?.prompt,
    messages.length,
    isLoading,
    append,
    setLastInteractionTime,
  ]);

  // Handle submit
  const handleSubmit = (e: React.FormEvent) => {
    window.electron.startPowerSaveBlocker();
    const customEvent = e as unknown as CustomEvent;
    const combinedTextFromInput = customEvent.detail?.value || '';
    const submittedSessionFiles = customEvent.detail?.sessionFiles || [];

    // Allow submission if there's text or session files
    const hasText = combinedTextFromInput.trim();
    const hasSessionFiles = submittedSessionFiles.length > 0;
    const hasContent = hasText || hasSessionFiles;

    if (hasContent) {
      setLastInteractionTime(Date.now());

      // Create user message with text (if any) and session files
      const userMessage = createUserMessage(
        hasText ? combinedTextFromInput.trim() : '',
        submittedSessionFiles // Use submitted session files
      );

      if (summarizedThread.length > 0) {
        resetMessagesWithSummary(
          messages,
          setMessages,
          ancestorMessages,
          setAncestorMessages,
          summaryContent
        );
        setTimeout(() => {
          append(userMessage);
          if (scrollRef.current?.scrollToBottom) {
            scrollRef.current.scrollToBottom();
          }
        }, 150);
      } else {
        append(userMessage);
        if (scrollRef.current?.scrollToBottom) {
          scrollRef.current.scrollToBottom();
        }
      }

      // Clear sessionFiles after sending the message
      setSessionFiles([]);
    } else {
      // If nothing was actually submitted (e.g. empty input and no images pasted)
      window.electron.stopPowerSaveBlocker();
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
      const textContent = lastMessage.content.find((c): c is TextContent => c.type === 'text');
      const textValue = textContent?.text || '';

      // Set the text back to the input field
      _setInput(textValue);

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
    return filteredMessages
      .reduce<string[]>((history, message) => {
        if (isUserMessage(message)) {
          const textContent = message.content.find((c): c is TextContent => c.type === 'text');
          const text = textContent?.text?.trim();
          if (text) {
            history.push(text);
          }
        }
        return history;
      }, [])
      .reverse();
  }, [filteredMessages]);

  // Fetch session metadata to get token count
  useEffect(() => {
    const fetchSessionTokens = async () => {
      try {
        const sessionDetails = await fetchSessionDetails(chat.id);
        setSessionTokenCount(sessionDetails.metadata.total_tokens || 0);
      } catch (err) {
        console.error('Error fetching session token count:', err);
      }
    };
    if (chat.id) {
      fetchSessionTokens();
    }
  }, [chat.id, messages]);

  const toolCallNotifications = notifications.reduce((map, item) => {
    const key = item.request_id;
    if (!map.has(key)) {
      map.set(key, []);
    }
    map.get(key).push(item);
    return map;
  }, new Map());

  const handleDrop = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    const files = Array.from(e.dataTransfer.files);
    const imageFiles = files.filter((file) => file.type.startsWith('image/'));
    const nonImageFiles = files.filter((file) => !file.type.startsWith('image/'));

    // Handle non-image files first - add them to sessionFiles
    if (nonImageFiles.length > 0) {
      const processNonImageFiles = async () => {
        // Collect all new session files first
        const newSessionFiles: SessionFile[] = [];

        for (const file of nonImageFiles) {
          try {
            // Get the file path using the electron API
            const filePath = window.electron.getPathForFile(file);
            if (filePath) {
              // Get the path type
              const pathType = await window.electron.getPathType(filePath);

              // Check if this path is already in sessionFiles
              const isAlreadyAdded = sessionFiles.some((item) => item.path === filePath);

              if (!isAlreadyAdded) {
                const newSessionFile: SessionFile = {
                  id: `file-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`,
                  path: filePath,
                  type: pathType === 'directory' ? 'directory' : 'file',
                };
                newSessionFiles.push(newSessionFile);
              }
            }
          } catch (error) {
            console.error('Error processing dropped file:', error);
          }
        }

        // Update sessionFiles with all new items at once
        if (newSessionFiles.length > 0) {
          setSessionFiles([...sessionFiles, ...newSessionFiles]);
        }
      };
      processNonImageFiles();
    }

    // Handle image files with the same logic as paste functionality
    if (imageFiles.length === 0) return;

    // Check if adding these images would exceed the limit
    if (sessionFiles.length + imageFiles.length > MAX_IMAGES_PER_MESSAGE) {
      // Show error message to user
      setSessionFiles((prev) => [
        ...prev,
        {
          id: `error-${Date.now()}`,
          path: '',
          type: 'image',
          dataUrl: '',
          isLoading: false,
          error: `Cannot drop ${imageFiles.length} image(s). Maximum ${MAX_IMAGES_PER_MESSAGE} images per message allowed.`,
        },
      ]);

      // Remove the error message after 3 seconds
      setTimeout(() => {
        setSessionFiles((prev) => prev.filter((file) => !file.id.startsWith('error-')));
      }, 3000);

      return;
    }

    for (const file of imageFiles) {
      // Check individual file size before processing
      if (file.size > MAX_IMAGE_SIZE_MB * 1024 * 1024) {
        const errorId = `error-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
        setSessionFiles((prev) => [
          ...prev,
          {
            id: errorId,
            path: '',
            type: 'image',
            dataUrl: '',
            isLoading: false,
            error: `Image too large (${Math.round(file.size / (1024 * 1024))}MB). Maximum ${MAX_IMAGE_SIZE_MB}MB allowed.`,
          },
        ]);

        // Remove the error message after 3 seconds
        setTimeout(() => {
          setSessionFiles((prev) => prev.filter((file) => file.id !== errorId));
        }, 3000);

        continue;
      }

      const reader = new FileReader();
      reader.onload = async (e) => {
        const dataUrl = e.target?.result as string;
        if (dataUrl) {
          const imageId = `img-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
          setSessionFiles((prev) => [
            ...prev,
            {
              id: imageId,
              path: '',
              type: 'image',
              dataUrl,
              isLoading: true,
            },
          ]);

          try {
            const result = await window.electron.saveDataUrlToTemp(dataUrl, imageId);
            setSessionFiles((prev) =>
              prev.map((file) =>
                file.id === result.id
                  ? { ...file, path: result.filePath || '', error: result.error, isLoading: false }
                  : file
              )
            );
          } catch (err) {
            console.error('Error saving dropped image:', err);
            setSessionFiles((prev) =>
              prev.map((file) =>
                file.id === imageId
                  ? { ...file, error: 'Failed to save image via Electron.', isLoading: false }
                  : file
              )
            );
          }
        }
      };
      reader.readAsDataURL(file);
    }
  };

  const handleDragOver = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
  };

  return (
    <CurrentModelContext.Provider value={currentModelInfo}>
      <div className="flex flex-col w-full h-screen items-center justify-center">
        {/* Loader when generating recipe */}
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
              activities={Array.isArray(recipeConfig?.activities) ? recipeConfig!.activities : null}
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
                        ) : (
                          <GooseMessage
                            messageHistoryIndex={chat?.messageHistoryIndex}
                            message={message}
                            messages={messages}
                            append={append}
                            appendMessage={(newMessage) => {
                              const updatedMessages = [...messages, newMessage];
                              setMessages(updatedMessages);
                            }}
                            toolCallNotifications={toolCallNotifications}
                          />
                        )}
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
              setView={setView}
              numTokens={sessionTokenCount}
              messages={messages}
              setMessages={setMessages}
              sessionFiles={sessionFiles}
              setSessionFiles={setSessionFiles}
            />
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
    </CurrentModelContext.Provider>
  );
}
