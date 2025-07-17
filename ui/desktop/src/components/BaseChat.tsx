/**
 * BaseChat Component
 *
 * BaseChat is the foundational chat component that provides the core conversational interface
 * for the Goose Desktop application. It serves as the shared base for both Hub and Pair components,
 * offering a flexible and extensible chat experience.
 *
 * Key Responsibilities:
 * - Manages the complete chat lifecycle (messages, input, submission, responses)
 * - Handles file drag-and-drop functionality with preview generation
 * - Integrates with multiple specialized hooks for chat engine, recipes, sessions, etc.
 * - Provides context management and session summarization capabilities
 * - Supports both user and assistant message rendering with tool call integration
 * - Manages loading states, error handling, and retry functionality
 * - Offers customization points through render props and configuration options
 *
 * Architecture:
 * - Uses a provider pattern (ChatContextManagerProvider) for state management
 * - Leverages composition through render props for flexible UI customization
 * - Integrates with multiple custom hooks for separation of concerns:
 *   - useChatEngine: Core chat functionality and API integration
 *   - useRecipeManager: Recipe/agent configuration management
 *   - useSessionContinuation: Session persistence and resumption
 *   - useFileDrop: Drag-and-drop file handling with previews
 *   - useCostTracking: Token usage and cost calculation
 *
 * Customization Points:
 * - renderHeader(): Custom header content (used by Hub for insights/recipe controls)
 * - renderBeforeMessages(): Content before message list (used by Hub for SessionInsights)
 * - renderAfterMessages(): Content after message list
 * - customChatInputProps: Props passed to ChatInput for specialized behavior
 * - customMainLayoutProps: Props passed to MainPanelLayout
 * - contentClassName: Custom CSS classes for the content area
 *
 * File Handling:
 * - Supports drag-and-drop of files with visual feedback
 * - Generates image previews for supported file types
 * - Integrates dropped files with chat input for seamless attachment
 * - Uses data-drop-zone="true" to designate safe drop areas
 *
 * The component is designed to be the single source of truth for chat functionality
 * while remaining flexible enough to support different UI contexts (Hub vs Pair).
 */

import React, { useEffect, useContext, createContext, useRef, useCallback } from 'react';
import { useLocation } from 'react-router-dom';
import { SearchView } from './conversation/SearchView';
import { AgentHeader } from './AgentHeader';
import LayingEggLoader from './LayingEggLoader';
import LoadingGoose from './LoadingGoose';
import Splash from './Splash';
import PopularChatTopics from './PopularChatTopics';
import ProgressiveMessageList from './ProgressiveMessageList';
import { SessionSummaryModal } from './context_management/SessionSummaryModal';
import {
  ChatContextManagerProvider,
  useChatContextManager,
} from './context_management/ChatContextManager';
import { type View, ViewOptions } from '../App';
import { MainPanelLayout } from './Layout/MainPanelLayout';
import ChatInput from './ChatInput';
import { ScrollArea, ScrollAreaHandle } from './ui/scroll-area';
import { useChatEngine } from '../hooks/useChatEngine';
import { useRecipeManager } from '../hooks/useRecipeManager';
import { useSessionContinuation } from '../hooks/useSessionContinuation';
import { useFileDrop } from '../hooks/useFileDrop';
import { useCostTracking } from '../hooks/useCostTracking';
import { Message } from '../types/message';
import { Recipe } from '../recipe';

// Context for sharing current model info
const CurrentModelContext = createContext<{ model: string; mode: string } | null>(null);
export const useCurrentModelInfo = () => useContext(CurrentModelContext);

export interface ChatType {
  id: string;
  title: string;
  messageHistoryIndex: number;
  messages: Message[];
  recipeConfig?: Recipe | null; // Add recipe configuration to chat state
}

interface BaseChatProps {
  chat: ChatType;
  setChat: (chat: ChatType) => void;
  setView: (view: View, viewOptions?: ViewOptions) => void;
  setIsGoosehintsModalOpen?: (isOpen: boolean) => void;
  enableLocalStorage?: boolean;
  onMessageStreamFinish?: () => void;
  onMessageSubmit?: (message: string) => void; // Callback after message is submitted
  renderHeader?: () => React.ReactNode;
  renderBeforeMessages?: () => React.ReactNode;
  renderAfterMessages?: () => React.ReactNode;
  customChatInputProps?: Record<string, unknown>;
  customMainLayoutProps?: Record<string, unknown>;
  contentClassName?: string; // Add custom class for content area
  disableSearch?: boolean; // Disable search functionality (for Hub)
  showPopularTopics?: boolean; // Show popular chat topics in empty state (for Pair)
  suppressEmptyState?: boolean; // Suppress empty state content (for transitions)
}

function BaseChatContent({
  chat,
  setChat,
  setView,
  setIsGoosehintsModalOpen,
  enableLocalStorage = false,
  onMessageStreamFinish,
  onMessageSubmit,
  renderHeader,
  renderBeforeMessages,
  renderAfterMessages,
  customChatInputProps = {},
  customMainLayoutProps = {},
  contentClassName = '',
  disableSearch = false,
  showPopularTopics = false,
  suppressEmptyState = false,
}: BaseChatProps) {
  const location = useLocation();
  const scrollRef = useRef<ScrollAreaHandle>(null);

  // Get disableAnimation from location state
  const disableAnimation = location.state?.disableAnimation || false;

  // Track if user has started using the current recipe
  const [hasStartedUsingRecipe, setHasStartedUsingRecipe] = React.useState(false);
  const [currentRecipeTitle, setCurrentRecipeTitle] = React.useState<string | null>(null);

  const {
    summaryContent,
    summarizedThread,
    isSummaryModalOpen,
    isLoadingSummary,
    resetMessagesWithSummary,
    closeSummaryModal,
    updateSummary,
  } = useChatContextManager();

  // Use shared chat engine
  const {
    messages,
    filteredMessages,
    ancestorMessages,
    setAncestorMessages,
    append,
    isLoading,
    error,
    setMessages,
    input: _input,
    setInput: _setInput,
    handleSubmit: engineHandleSubmit,
    onStopGoose,
    sessionTokenCount,
    sessionInputTokens,
    sessionOutputTokens,
    localInputTokens,
    localOutputTokens,
    commandHistory,
    toolCallNotifications,
    updateMessageStreamBody,
    sessionMetadata,
    isUserMessage,
  } = useChatEngine({
    chat,
    setChat,
    onMessageStreamFinish: () => {
      // Auto-scroll to bottom when message stream finishes
      setTimeout(() => {
        if (scrollRef.current?.scrollToBottom) {
          scrollRef.current.scrollToBottom();
        }
      }, 300);

      // Call the original callback if provided
      onMessageStreamFinish?.();
    },
    onMessageSent: () => {
      // Mark that user has started using the recipe
      if (recipeConfig) {
        setHasStartedUsingRecipe(true);
      }

      // Create new session after message is sent if needed
      createNewSessionIfNeeded();
    },
    enableLocalStorage,
  });

  // Use shared recipe manager
  const {
    recipeConfig,
    initialPrompt,
    isGeneratingRecipe,
    handleAutoExecution,
    recipeError,
    setRecipeError,
  } = useRecipeManager(messages, location.state);

  // Reset recipe usage tracking when recipe changes
  useEffect(() => {
    if (recipeConfig?.title !== currentRecipeTitle) {
      setCurrentRecipeTitle(recipeConfig?.title || null);
      setHasStartedUsingRecipe(false);

      // Clear existing messages when a new recipe is loaded
      if (recipeConfig?.title && recipeConfig.title !== currentRecipeTitle) {
        setMessages([]);
        setAncestorMessages([]);
      }
    }
  }, [recipeConfig?.title, currentRecipeTitle, setMessages, setAncestorMessages]);

  // Handle recipe auto-execution
  useEffect(() => {
    handleAutoExecution(append, isLoading);
  }, [handleAutoExecution, append, isLoading]);

  // Use shared session continuation
  const { createNewSessionIfNeeded } = useSessionContinuation({
    chat,
    setChat,
    summarizedThread,
    updateMessageStreamBody,
  });

  // Use shared file drop
  const { droppedFiles, setDroppedFiles, handleDrop, handleDragOver } = useFileDrop();

  // Use shared cost tracking
  const { sessionCosts } = useCostTracking({
    sessionInputTokens,
    sessionOutputTokens,
    localInputTokens,
    localOutputTokens,
    sessionMetadata,
  });

  useEffect(() => {
    // Log all messages when the component first mounts
    window.electron.logInfo(
      'Initial messages when resuming session: ' + JSON.stringify(chat.messages, null, 2)
    );
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []); // Empty dependency array means this runs once on mount

  // Handle submit with summary reset support
  const handleSubmit = (e: React.FormEvent) => {
    const customEvent = e as unknown as CustomEvent;
    const combinedTextFromInput = customEvent.detail?.value || '';

    // Mark that user has started using the recipe when they submit a message
    if (recipeConfig && combinedTextFromInput.trim()) {
      setHasStartedUsingRecipe(true);
    }

    const onSummaryReset =
      summarizedThread.length > 0
        ? () => {
            resetMessagesWithSummary(
              messages,
              setMessages,
              ancestorMessages,
              setAncestorMessages,
              summaryContent
            );
          }
        : undefined;

    // Call the callback if provided (for Hub to handle navigation)
    if (onMessageSubmit && combinedTextFromInput.trim()) {
      onMessageSubmit(combinedTextFromInput);
    }

    engineHandleSubmit(combinedTextFromInput, onSummaryReset);

    // Auto-scroll to bottom after submitting
    if (onSummaryReset) {
      // If we're resetting with summary, delay the scroll a bit more
      setTimeout(() => {
        if (scrollRef.current?.scrollToBottom) {
          scrollRef.current.scrollToBottom();
        }
      }, 200);
    } else {
      // Immediate scroll for regular submit
      if (scrollRef.current?.scrollToBottom) {
        scrollRef.current.scrollToBottom();
      }
    }
  };

  // Wrapper for append that tracks recipe usage
  const appendWithTracking = (text: string | Message) => {
    // Mark that user has started using the recipe when they use append
    if (recipeConfig) {
      setHasStartedUsingRecipe(true);
    }
    append(text);
  };
  // Callback to handle scroll to bottom from ProgressiveMessageList
  const handleScrollToBottom = useCallback(() => {
    setTimeout(() => {
      if (scrollRef.current?.scrollToBottom) {
        scrollRef.current.scrollToBottom();
      }
    }, 100);
  }, []);

  return (
    <div className="h-full flex flex-col min-h-0">
      <MainPanelLayout
        backgroundColor={'bg-background-muted'}
        removeTopPadding={true}
        {...customMainLayoutProps}
      >
        {/* Loader when generating recipe */}
        {isGeneratingRecipe && <LayingEggLoader />}

        {/* Custom header */}
        {renderHeader && renderHeader()}

        {/* Chat container with sticky recipe header */}
        <div className="flex flex-col flex-1 mb-0.5 min-h-0 relative">
          <ScrollArea
            ref={scrollRef}
            className={`flex-1 bg-background-default rounded-b-2xl min-h-0 relative ${contentClassName}`}
            autoScroll
            onDrop={handleDrop}
            onDragOver={handleDragOver}
            data-drop-zone="true"
            paddingX={6}
            paddingY={0}
          >
            {/* Recipe agent header - sticky at top of chat container */}
            {recipeConfig?.title && (
              <div className="sticky top-0 z-10 bg-background-default px-0 -mx-6 mb-6 pt-6">
                <AgentHeader
                  title={recipeConfig.title}
                  profileInfo={
                    recipeConfig.profile
                      ? `${recipeConfig.profile} - ${recipeConfig.mcps || 12} MCPs`
                      : undefined
                  }
                  onChangeProfile={() => {
                    console.log('Change profile clicked');
                  }}
                  showBorder={true}
                />
              </div>
            )}

            {/* Custom content before messages */}
            {renderBeforeMessages && renderBeforeMessages()}

            {/* Messages or Splash or Popular Topics */}
            {
              // Check if we should show splash instead of messages
              (() => {
                // Show splash if we have a recipe and user hasn't started using it yet
                const shouldShowSplash =
                  recipeConfig && !hasStartedUsingRecipe && !suppressEmptyState;

                return shouldShowSplash;
              })() ? (
                <>
                  {/* Show Splash when we have a recipe config and user hasn't started using it */}
                  {recipeConfig ? (
                    <Splash
                      append={(text: string) => appendWithTracking(text)}
                      activities={
                        Array.isArray(recipeConfig.activities) ? recipeConfig.activities : null
                      }
                      title={recipeConfig.title}
                    />
                  ) : showPopularTopics ? (
                    /* Show PopularChatTopics when no real messages, no recipe, and showPopularTopics is true (Pair view) */
                    <PopularChatTopics append={(text: string) => appendWithTracking(text)} />
                  ) : null}
                </>
              ) : filteredMessages.length > 0 || (recipeConfig && hasStartedUsingRecipe) ? (
                <>
                  {disableSearch ? (
                    // Render messages without SearchView wrapper when search is disabled
                    <ProgressiveMessageList
                      messages={filteredMessages}
                      chat={chat}
                      toolCallNotifications={toolCallNotifications}
                      append={append}
                      appendMessage={(newMessage) => {
                        const updatedMessages = [...messages, newMessage];
                        setMessages(updatedMessages);
                      }}
                      isUserMessage={isUserMessage}
                      onScrollToBottom={handleScrollToBottom}
                      isStreamingMessage={isLoading}
                    />
                  ) : (
                    // Render messages with SearchView wrapper when search is enabled
                    <SearchView>
                      <ProgressiveMessageList
                        messages={filteredMessages}
                        chat={chat}
                        toolCallNotifications={toolCallNotifications}
                        append={append}
                        appendMessage={(newMessage) => {
                          const updatedMessages = [...messages, newMessage];
                          setMessages(updatedMessages);
                        }}
                        isUserMessage={isUserMessage}
                        onScrollToBottom={handleScrollToBottom}
                        isStreamingMessage={isLoading}
                      />
                    </SearchView>
                  )}

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
              ) : showPopularTopics ? (
                /* Show PopularChatTopics when no messages, no recipe, and showPopularTopics is true (Pair view) */
                <PopularChatTopics append={(text: string) => append(text)} />
              ) : null /* Show nothing when messages.length === 0 && suppressEmptyState === true */
            }

            {/* Custom content after messages */}
            {renderAfterMessages && renderAfterMessages()}
          </ScrollArea>

          {/* Fixed loading indicator at bottom left of chat container */}
          {isLoading && (
            <div className="absolute bottom-1 left-4 z-20 pointer-events-none">
              <LoadingGoose message={isLoadingSummary ? 'summarizing conversation…' : undefined} />
            </div>
          )}
        </div>

        <div
          className={`relative z-10 ${disableAnimation ? '' : 'animate-[fadein_400ms_ease-in_forwards]'}`}
        >
          <ChatInput
            handleSubmit={handleSubmit}
            isLoading={isLoading}
            onStop={onStopGoose}
            commandHistory={commandHistory}
            initialValue={_input || initialPrompt}
            setView={setView}
            numTokens={sessionTokenCount}
            inputTokens={sessionInputTokens || localInputTokens}
            outputTokens={sessionOutputTokens || localOutputTokens}
            droppedFiles={droppedFiles}
            onFilesProcessed={() => setDroppedFiles([])} // Clear dropped files after processing
            messages={messages}
            setMessages={setMessages}
            disableAnimation={disableAnimation}
            sessionCosts={sessionCosts}
            setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
            recipeConfig={recipeConfig}
            {...customChatInputProps}
          />
        </div>
      </MainPanelLayout>

      <SessionSummaryModal
        isOpen={isSummaryModalOpen}
        onClose={closeSummaryModal}
        onSave={(editedContent) => {
          updateSummary(editedContent);
          closeSummaryModal();
        }}
        summaryContent={summaryContent}
      />

      {/* Recipe Error Modal */}
      {recipeError && (
        <div className="fixed inset-0 z-[300] flex items-center justify-center bg-black/50">
          <div className="bg-background-default border border-borderSubtle rounded-lg p-6 w-96 max-w-[90vw]">
            <h3 className="text-lg font-medium text-textProminent mb-4">Recipe Creation Failed</h3>
            <p className="text-textStandard mb-6">{recipeError}</p>
            <div className="flex justify-end">
              <button
                onClick={() => setRecipeError(null)}
                className="px-4 py-2 bg-textProminent text-bgApp rounded-lg hover:bg-opacity-90 transition-colors"
              >
                OK
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default function BaseChat(props: BaseChatProps) {
  return (
    <ChatContextManagerProvider>
      <BaseChatContent {...props} />
    </ChatContextManagerProvider>
  );
}
