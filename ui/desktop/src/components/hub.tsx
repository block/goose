import React, { useEffect, useRef, useState, useMemo, useContext, createContext } from 'react';
import { useLocation } from 'react-router-dom';
import { getApiUrl } from '../config';
import FlappyGoose from './FlappyGoose';
import GooseMessage from './GooseMessage';
import { type View, ViewOptions } from '../App';
import { ScrollAreaHandle } from './ui/scroll-area';
import UserMessage from './UserMessage';
import { SearchView } from './conversation/SearchView';
import { createRecipe } from '../recipe';
import { AgentHeader } from './AgentHeader';
import LayingEggLoader from './LayingEggLoader';
import { fetchSessionDetails, generateSessionId } from '../sessions';
import 'react-toastify/dist/ReactToastify.css';
import { useMessageStream } from '../hooks/useMessageStream';
import { SessionSummaryModal } from './context_management/SessionSummaryModal';
import { useModelAndProvider } from './ModelAndProviderContext';
import { getCostForModel } from '../utils/costDatabase';
import { Recipe } from '../recipe';
import {
  ChatContextManagerProvider,
  useChatContextManager,
} from './context_management/ChatContextManager';
import { ContextHandler } from './context_management/ContextHandler';
import { useChatContext } from '../contexts/ChatContext';
import {
  Message,
  createUserMessage,
  ToolCall,
  ToolCallResult,
  ToolRequestMessageContent,
  ToolResponseMessageContent,
  ToolConfirmationRequestMessageContent,
  TextContent,
  getTextContent,
} from '../types/message';
import { SessionInsights } from './sessions/SessionsInsights';
import { Button } from './ui/button';
import { Idea } from './icons';
import { Tooltip, TooltipContent, TooltipTrigger } from './ui/Tooltip';
import { Bot, Save, Send } from 'lucide-react';
import { MainPanelLayout } from './Layout/MainPanelLayout';
import ChatInput from './ChatInput';

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

export default function Hub({
  readyForAutoUserPrompt,
  chat,
  setChat,
  setView,
  setIsGoosehintsModalOpen,
}: {
  readyForAutoUserPrompt: boolean;
  chat: ChatType;
  setChat: (chat: ChatType) => void;
  setView: (view: View, viewOptions?: ViewOptions) => void;
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
}) {
  return (
    <ChatContextManagerProvider>
      <HubContent
        readyForAutoUserPrompt={readyForAutoUserPrompt}
        chat={chat}
        setChat={setChat}
        setView={setView}
        setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
      />
    </ChatContextManagerProvider>
  );
}

function HubContent({
  chat,
  setChat,
  setView,
  setIsGoosehintsModalOpen,
}: {
  readyForAutoUserPrompt: boolean;
  chat: ChatType;
  setChat: (chat: ChatType) => void;
  setView: (view: View, viewOptions?: ViewOptions) => void;
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
}) {
  return (
    <HubContentWithSidebar
      chat={chat}
      setChat={setChat}
      setView={setView}
      setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
    />
  );
}

function HubContentWithSidebar({
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
  const location = useLocation();
  const { resetChat } = useChatContext();

  const [lastInteractionTime, setLastInteractionTime] = useState<number>(Date.now());
  const [showGame, setShowGame] = useState(false);
  const [isGeneratingRecipe, setIsGeneratingRecipe] = useState(false);
  const [sessionTokenCount, setSessionTokenCount] = useState<number>(0);
  const [ancestorMessages, setAncestorMessages] = useState<Message[]>([]);
  const [droppedFiles, setDroppedFiles] = useState<string[]>([]);
  const [isInPairMode, setIsInPairMode] = useState(false);
  const [sessionInputTokens, setSessionInputTokens] = useState<number>(0);
  const [sessionOutputTokens, setSessionOutputTokens] = useState<number>(0);
  const [localInputTokens, setLocalInputTokens] = useState<number>(0);
  const [localOutputTokens, setLocalOutputTokens] = useState<number>(0);
  const [sessionCosts, setSessionCosts] = useState<{
    [key: string]: {
      inputTokens: number;
      outputTokens: number;
      totalCost: number;
    };
  }>({});
  const { currentModel, currentProvider } = useModelAndProvider();
  const prevModelRef = useRef<string | undefined>();
  const prevProviderRef = useRef<string | undefined>();

  // New state to force showing insights when navigating to hub
  const [forceShowInsights, setForceShowInsights] = useState(true);

  // Effect to detect direct navigation to the hub page
  useEffect(() => {
    // Check if we're on the hub page (root route)
    const isHubPage = location.pathname === '/';

    // If we're on the hub page, force showing insights only if there are no messages
    // This allows resumed sessions to display immediately
    if (isHubPage) {
      setForceShowInsights(chat.messages.length === 0);
      setIsInPairMode(false);
    }
  }, [location.pathname, chat.messages.length]);

  // Get disableAnimation from location state
  const disableAnimation = location.state?.disableAnimation || false;

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

  useEffect(() => {
    // Log all messages when the component first mounts
    window.electron.logInfo(
      'Initial messages when resuming session: ' + JSON.stringify(chat.messages, null, 2)
    );
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []); // Empty dependency array means this runs once on mount;

  // Get recipeConfig directly from appConfig
  const recipeConfig = window.appConfig.get('recipeConfig') as Recipe | null;

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
    updateMessageStreamBody,
    notifications,
    sessionMetadata,
  } = useMessageStream({
    api: getApiUrl('/reply'),
    id: chat.id,
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

      // Always emit refresh event when message stream finishes for new sessions
      // Check if this is a new session by looking at the current session ID format
      const isNewSession = chat.id && chat.id.match(/^\d{8}_\d{6}$/);
      if (isNewSession) {
        console.log(
          'Hub: Message stream finished for new session, emitting message-stream-finished event'
        );
        // Emit event to trigger session refresh
        window.dispatchEvent(new CustomEvent('message-stream-finished'));
      }
    },
  });

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

  // Simple token estimation function (roughly 4 characters per token)
  const estimateTokens = (text: string): number => {
    return Math.ceil(text.length / 4);
  };

  // Calculate token counts from messages
  useEffect(() => {
    let inputTokens = 0;
    let outputTokens = 0;

    messages.forEach((message) => {
      const textContent = getTextContent(message);
      if (textContent) {
        const tokens = estimateTokens(textContent);
        if (message.role === 'user') {
          inputTokens += tokens;
        } else if (message.role === 'assistant') {
          outputTokens += tokens;
        }
      }
    });

    setLocalInputTokens(inputTokens);
    setLocalOutputTokens(outputTokens);
  }, [messages]);

  // Update chat messages when they change and save to sessionStorage
  useEffect(() => {
    // @ts-expect-error - TypeScript being overly strict about the return type
    setChat((prevChat: ChatType) => ({ ...prevChat, messages }));
  }, [messages, setChat]);

  // Pre-fill input with recipe prompt instead of auto-sending it
  const initialPrompt = useMemo(() => {
    return recipeConfig?.prompt || '';
  }, [recipeConfig?.prompt]);

  // Handle submit
  const handleSubmit = (e: React.FormEvent) => {
    window.electron.startPowerSaveBlocker();
    const customEvent = e as unknown as CustomEvent;
    // ChatInput now sends a single 'value' field with text and appended image paths
    const combinedTextFromInput = customEvent.detail?.value || '';

    if (combinedTextFromInput.trim()) {
      setLastInteractionTime(Date.now());

      // createUserMessage was reverted to only accept text.
      // It will create a Message with a single TextContent part containing text + paths.
      const userMessage = createUserMessage(combinedTextFromInput.trim());

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
    } else {
      // If nothing was actually submitted (e.g. empty input and no images pasted)
      window.electron.stopPowerSaveBlocker();
    }
  };

  // Handle submit for the initial ChatInput (when no messages)
  const handleInitialSubmit = (e: React.FormEvent) => {
    const customEvent = e as unknown as CustomEvent;
    const combinedTextFromInput = customEvent.detail?.value || '';

    if (combinedTextFromInput.trim()) {
      // Navigate to pair page with animation disabled
      setView('pair', { disableAnimation: true });

      // Create and submit the user message
      const userMessage = createUserMessage(combinedTextFromInput.trim());
      append(userMessage);

      // Scroll to bottom after a short delay to ensure the message is rendered
      setTimeout(() => {
        if (scrollRef.current?.scrollToBottom) {
          scrollRef.current.scrollToBottom();
        }
      }, 100);
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

    // Check if there are any messages before proceeding
    if (!lastMessage) {
      return;
    }

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
        setSessionInputTokens(sessionDetails.metadata.accumulated_input_tokens || 0);
        setSessionOutputTokens(sessionDetails.metadata.accumulated_output_tokens || 0);
      } catch (err) {
        console.error('Error fetching session token count:', err);
      }
    };
    if (chat.id) {
      fetchSessionTokens();
    }
  }, [chat.id, messages]);

  // Update token counts when sessionMetadata changes from the message stream
  useEffect(() => {
    console.log('Session metadata received:', sessionMetadata);
    if (sessionMetadata) {
      setSessionTokenCount(sessionMetadata.totalTokens || 0);
      setSessionInputTokens(sessionMetadata.accumulatedInputTokens || 0);
      setSessionOutputTokens(sessionMetadata.accumulatedOutputTokens || 0);
    }
  }, [sessionMetadata]);

  // Handle model changes and accumulate costs
  useEffect(() => {
    if (
      prevModelRef.current !== undefined &&
      prevProviderRef.current !== undefined &&
      (prevModelRef.current !== currentModel || prevProviderRef.current !== currentProvider)
    ) {
      // Model/provider has changed, save the costs for the previous model
      const prevKey = `${prevProviderRef.current}/${prevModelRef.current}`;

      // Get pricing info for the previous model
      const prevCostInfo = getCostForModel(prevProviderRef.current, prevModelRef.current);

      if (prevCostInfo) {
        const prevInputCost =
          (sessionInputTokens || localInputTokens) * (prevCostInfo.input_token_cost || 0);
        const prevOutputCost =
          (sessionOutputTokens || localOutputTokens) * (prevCostInfo.output_token_cost || 0);
        const prevTotalCost = prevInputCost + prevOutputCost;

        // Save the accumulated costs for this model
        setSessionCosts((prev) => ({
          ...prev,
          [prevKey]: {
            inputTokens: sessionInputTokens || localInputTokens,
            outputTokens: sessionOutputTokens || localOutputTokens,
            totalCost: prevTotalCost,
          },
        }));
      }

      // Restore token counters from session metadata instead of resetting to 0
      // This preserves the accumulated session tokens when switching models
      // and ensures cost tracking remains accurate across model changes
      if (sessionMetadata) {
        // Use Math.max to ensure non-negative values and handle potential data issues
        setSessionTokenCount(Math.max(0, sessionMetadata.totalTokens || 0));
        setSessionInputTokens(Math.max(0, sessionMetadata.accumulatedInputTokens || 0));
        setSessionOutputTokens(Math.max(0, sessionMetadata.accumulatedOutputTokens || 0));
      } else {
        // Fallback: if no session metadata, preserve current session tokens instead of resetting
        // This handles edge cases where metadata might not be available yet
        console.warn(
          'No session metadata available during model change, preserving current tokens'
        );
      }
      // Only reset local token estimation counters since they're model-specific
      setLocalInputTokens(0);
      setLocalOutputTokens(0);

      console.log(
        'Model changed from',
        `${prevProviderRef.current}/${prevModelRef.current}`,
        'to',
        `${currentProvider}/${currentModel}`,
        '- saved costs and restored session token counters'
      );
    }

    prevModelRef.current = currentModel || undefined;
    prevProviderRef.current = currentProvider || undefined;
  }, [
    currentModel,
    currentProvider,
    sessionInputTokens,
    sessionOutputTokens,
    localInputTokens,
    localOutputTokens,
    sessionMetadata,
  ]);

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

  const toolCallNotifications = notifications.reduce((map, item) => {
    const key = item.request_id;
    if (!map.has(key)) {
      map.set(key, []);
    }
    map.get(key).push(item);
    return map;
  }, new Map());

  return (
    <div>
      <MainPanelLayout>
        {/* Loader when generating recipe */}
        {isGeneratingRecipe && <LayingEggLoader />}

        <div className="h-12 flex items-center justify-between absolute">
          <div className="flex items-center justify-end pr-4">
            {messages.length > 0 && (
              <>
                {setIsGoosehintsModalOpen && (
                  <Tooltip delayDuration={500}>
                    <TooltipTrigger className="w-full">
                      <Button
                        onClick={() => setIsGoosehintsModalOpen(true)}
                        className="px-3"
                        variant="ghost"
                        size="sm"
                        shape="round"
                      >
                        <div className="flex gap-2 items-center text-text-default">
                          <Idea className="w-4 h-4" />
                          {/* Configure .goosehints */}
                        </div>
                      </Button>
                    </TooltipTrigger>
                    <TooltipContent side="right">
                      <p>Customize instructions</p>
                    </TooltipContent>
                  </Tooltip>
                )}

                {recipeConfig ? (
                  <>
                    <Tooltip delayDuration={500}>
                      <TooltipTrigger className="w-full">
                        <Button
                          onClick={() => {
                            window.electron.createChatWindow(
                              undefined,
                              undefined,
                              undefined,
                              undefined,
                              recipeConfig as Recipe,
                              'recipeEditor'
                            );
                          }}
                          className="px-3"
                          variant="ghost"
                        >
                          <div className="flex gap-2 items-center text-text-default">
                            <Send className="w-4 h-4" />
                            View recipe
                          </div>
                        </Button>
                      </TooltipTrigger>
                      <TooltipContent side="right">
                        <p>View the recipe you're using</p>
                      </TooltipContent>
                    </Tooltip>

                    <Tooltip delayDuration={500}>
                      <TooltipTrigger className="w-full">
                        <Button
                          // onClick={handleSaveRecipeClick}
                          className="px-3"
                          variant="ghost"
                          size="sm"
                          shape="round"
                        >
                          <div className="flex gap-2 items-center text-text-default">
                            <Save className="w-4 h-4" />
                            Save recipe
                          </div>
                        </Button>
                      </TooltipTrigger>
                      <TooltipContent side="right">
                        <p>Save this recipe for reuse</p>
                      </TooltipContent>
                    </Tooltip>
                  </>
                ) : (
                  <Tooltip delayDuration={500}>
                    <TooltipTrigger className="w-full">
                      <Button
                        onClick={() => {
                          window.electron.logInfo('Make Agent button clicked');
                          window.dispatchEvent(new CustomEvent('make-agent-from-chat'));
                        }}
                        className="px-3"
                        variant="ghost"
                        size="sm"
                        shape="round"
                      >
                        <div className="flex gap-2 items-center text-text-default">
                          <Bot className="w-4 h-4" />
                          {/* Make Agent from this session */}
                        </div>
                      </Button>
                    </TooltipTrigger>
                    <TooltipContent side="right">
                      <p>Make a custom agent you can share or reuse</p>
                    </TooltipContent>
                  </Tooltip>
                )}
              </>
            )}
          </div>
        </div>

        <div
          className={`flex flex-col min-w-0 flex-1 overflow-y-scroll relative`}
          onDrop={handleDrop}
          onDragOver={handleDragOver}
        >
          {/* Session Insights - always show on hub page regardless of message count */}
          {(messages.length === 0 || forceShowInsights) && !isInPairMode && <SessionInsights />}

          {/* Only show agent header and messages if insights are not forced */}
          {!forceShowInsights && recipeConfig?.title && messages.length > 0 && (
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
          {(messages.length === 0 || forceShowInsights) && !isInPairMode ? (
            <>
              <SearchView>{/* Empty search view when no messages or insights forced */}</SearchView>
            </>
          ) : (
            <>
              <SearchView>
                {filteredMessages.map((message, index) => {
                  const isUser = isUserMessage(message);
                  // const nextMessage = filteredMessages[index + 1];

                  return (
                    <div
                      key={message.id || index}
                      className={`relative ${index === 0 ? 'mt-0' : 'mt-4'} ${isUser ? 'user' : 'assistant'}`}
                      data-testid="message-container"
                    >
                      {isUser ? (
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
                  );
                })}
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
            </>
          )}
        </div>
        <div
          className={`relative z-10 ${disableAnimation ? '' : 'animate-[fadein_400ms_ease-in_forwards]'}`}
        >
          {/* Show session continuation UI when there's an active session but insights are shown */}
          {forceShowInsights && messages.length > 0 ? (
            <div className="mx-6 mb-6 p-4 rounded-xl border border-borderSubtle bg-background-default">
              <div className="flex flex-col items-center gap-4">
                <div className="text-center">
                  <h3 className="text-lg font-medium">You have an active session</h3>
                  <p className="text-sm text-textSubtle mt-1">
                    Would you like to continue or start fresh?
                  </p>
                </div>
                <div className="flex gap-3">
                  <Button
                    onClick={() => {
                      // Navigate to pair page with the session
                      setView('pair');
                    }}
                    variant="default"
                  >
                    Continue Session
                  </Button>
                  <Button
                    onClick={() => {
                      resetChat();
                      setForceShowInsights(false);
                    }}
                    variant="outline"
                  >
                    Start New Session
                  </Button>
                </div>
              </div>
            </div>
          ) : (
            /* ChatInput for all conversations */
            <ChatInput
              handleSubmit={
                forceShowInsights || (messages.length === 0 && !isInPairMode)
                  ? handleInitialSubmit
                  : handleSubmit
              }
              isLoading={isLoading}
              onStop={onStopGoose}
              commandHistory={commandHistory}
              initialValue={_input || initialPrompt}
              setView={setView}
              numTokens={sessionTokenCount}
              inputTokens={sessionInputTokens || localInputTokens}
              outputTokens={sessionOutputTokens || localOutputTokens}
              droppedFiles={droppedFiles}
              messages={forceShowInsights ? [] : messages}
              setMessages={setMessages}
              disableAnimation={disableAnimation}
              sessionCosts={sessionCosts}
            />
          )}
        </div>
      </MainPanelLayout>

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
