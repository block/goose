/**
 * BaseChat Component
 * 
 * A comprehensive chat interface component that serves as the foundation for all chat experiences
 * in the Goose Desktop application. This component handles the complete chat lifecycle including
 * message rendering, user input, file handling, and advanced features like recipe integration,
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
 * - **NEW**: Intelligent scrolling that respects user reading behavior
 *
 * Architecture:
 * - Uses a provider pattern (ChatContextManagerProvider) for state management
 * - Leverages composition through render props for flexible UI customization
 * - Integrates with multiple custom hooks for separation of concerns:
 *   - useChatEngine: Core chat functionality and API integration
 *   - useRecipeManager: Recipe/agent configuration management
 *   - useFileDrop: Drag-and-drop file handling with previews
 *   - useCostTracking: Token usage and cost calculation
 *   - **NEW**: Intelligent scrolling system for better UX
 *
 * Customization Points:
 * - renderHeader(): Custom header content (used by Hub for insights/recipe controls)
 * - renderBeforeMessages(): Content before message list (used by Hub for SessionInsights)
 * - renderAfterMessages(): Content after message list
 * - customChatInputProps: Props passed to ChatInput for specialized behavior
 * - customMainLayoutProps: Props passed to MainPanelLayout
 * - contentClassName: Custom CSS classes for the content area
 * - **NEW**: intelligentScrolling: Enable smart scroll behavior (default: true)
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

import React, { useEffect, useRef } from 'react';
import { useLocation } from 'react-router-dom';
import { AgentHeader } from './AgentHeader';
import LayingEggLoader from './LayingEggLoader';
import LoadingGoose from './LoadingGoose';
import RecipeActivities from './recipes/RecipeActivities';
import PopularChatTopics from './PopularChatTopics';
import ProgressiveMessageList from './ProgressiveMessageList';
import { View, ViewOptions } from '../utils/navigationUtils';
import { ContextManagerProvider, useContextManager } from './context_management/ContextManager';
import { MainPanelLayout } from './Layout/MainPanelLayout';
import ChatInput from './ChatInput';
import { ScrollAreaEnhanced, ScrollAreaHandle } from './ui/scroll-area-enhanced';
import { RecipeWarningModal } from './ui/RecipeWarningModal';
import ParameterInputModal from './ParameterInputModal';
import { useChatEngine } from '../hooks/useChatEngine';
import { useRecipeManager } from '../hooks/useRecipeManager';
import { useFileDrop } from '../hooks/useFileDrop';
import { useCostTracking } from '../hooks/useCostTracking';
import { Message } from '../types/message';
import { ChatState } from '../types/chatState';
import { ChatType } from '../types/chat';
import { useToolCount } from './alerts/useToolCount';

interface BaseChatProps {
  chat: ChatType;
  setChat: (chat: ChatType) => void;
  setView?: (view: View, options?: ViewOptions) => void;
  setIsGoosehintsModalOpen?: (open: boolean) => void;
  onMessageStreamFinish?: () => void;
  renderHeader?: () => React.ReactNode;
  renderBeforeMessages?: () => React.ReactNode;
  renderAfterMessages?: () => React.ReactNode;
  customChatInputProps?: Record<string, unknown>;
  customMainLayoutProps?: Record<string, unknown>;
  contentClassName?: string;
  disableSearch?: boolean;
  showPopularTopics?: boolean;
  suppressEmptyState?: boolean;
  autoSubmit?: boolean;
  loadingChat: boolean;
  // NEW: Intelligent scrolling configuration
  intelligentScrolling?: boolean; // Enable intelligent scrolling (default: true)
  scrollConfig?: {
    idleTimeout?: number; // Time to wait before considering user idle (default: 4000ms)
    activityDebounce?: number; // Debounce for activity detection (default: 100ms)
    scrollVelocityThreshold?: number; // Minimum velocity for intentional scroll (default: 0.5)
    autoScrollDelay?: number; // Delay before auto-scroll when idle at bottom (default: 200ms)
    gracefulReturnDelay?: number; // Delay before graceful return when idle above (default: 1500ms)
  };
}

function BaseChatContent({
  chat,
  setChat,
  setView,
  setIsGoosehintsModalOpen,
  onMessageStreamFinish,
  renderHeader,
  renderBeforeMessages,
  renderAfterMessages,
  customChatInputProps = {},
  customMainLayoutProps = {},
  contentClassName = '',
  disableSearch = false,
  showPopularTopics = false,
  suppressEmptyState = false,
  autoSubmit = false,
  loadingChat = false,
  intelligentScrolling = true, // Enable by default
  scrollConfig = {},
}: BaseChatProps) {
  const location = useLocation();
  const scrollRef = useRef<ScrollAreaHandle>(null);

  const disableAnimation = location.state?.disableAnimation || false;
  const [hasStartedUsingRecipe, setHasStartedUsingRecipe] = React.useState(false);
  const [currentRecipeTitle, setCurrentRecipeTitle] = React.useState<string | null>(null);
  const { isCompacting, handleManualCompaction } = useContextManager();

  // Legacy timeout ref for backward compatibility (when intelligent scrolling is disabled)
  const autoScrollTimeoutRef = useRef<number | null>(null);
  const wasFollowingRef = useRef<boolean>(true);

  // Legacy isNearBottom function for backward compatibility
  const isNearBottom = React.useCallback(() => {
    if (!scrollRef.current) return false;

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const viewport = scrollRef.current as any;
    if (!viewport.viewportRef?.current) return false;

    const viewportElement = viewport.viewportRef.current;
    const { scrollHeight, scrollTop, clientHeight } = viewportElement;
    const scrollBottom = scrollTop + clientHeight;
    const distanceFromBottom = scrollHeight - scrollBottom;

    return distanceFromBottom <= 100;
  }, []);

  // Legacy conditional auto-scroll function (used when intelligent scrolling is disabled)
  const legacyConditionalAutoScroll = React.useCallback(() => {
    if (intelligentScrolling) return; // Skip legacy behavior when intelligent scrolling is enabled

    // Clear any existing timeout
    if (autoScrollTimeoutRef.current) {
      clearTimeout(autoScrollTimeoutRef.current);
    }

    // Debounce the auto-scroll to prevent jumpy behavior and prevent multiple rapid scrolls
    autoScrollTimeoutRef.current = window.setTimeout(() => {
      // Only auto-scroll if user was following when the agent started responding
      if (wasFollowingRef.current && scrollRef.current) {
        scrollRef.current.scrollToBottom();
      }
    }, 150);
  }, [intelligentScrolling]);

  // CRITICAL FIX: Remove handleNewMessage entirely - it's not needed with intelligent scrolling
  // The intelligent scroll system handles all message updates via content change detection
  // This prevents the bypass that was causing the lock to break

  useEffect(() => {
    return () => {
      if (autoScrollTimeoutRef.current) {
        clearTimeout(autoScrollTimeoutRef.current);
      }
    };
  }, []);

  // Use shared chat engine
  const {
    messages,
    setMessages,
    input,
    setInput,
    append,
    isLoading,
    stop,
    reload,
    chatState,
    error,
    sessionTokenCount,
    sessionInputTokens,
    sessionOutputTokens,
    localInputTokens,
    localOutputTokens,
    commandHistory,
    toolCallNotifications,
    sessionMetadata,
    isUserMessage,
    clearError,
    onMessageUpdate,
  } = useChatEngine({
    chat,
    setChat,
    onMessageStreamFinish: () => {
      // CRITICAL FIX: Completely remove the bypass logic
      // Let the intelligent scroll system handle everything via content change detection
      console.log('📝 Message stream finished - intelligent scrolling handles all updates');

      // Call the original callback if provided
      onMessageStreamFinish?.();
    },
    onMessageSent: () => {
      if (!intelligentScrolling) {
        wasFollowingRef.current = isNearBottom();
      }

      // Mark that user has started using the recipe
      if (recipeConfig) {
        setHasStartedUsingRecipe(true);
      }
    },
  });

  // Use shared recipe manager
  const {
    recipeConfig,
    filteredParameters,
    initialPrompt,
    isGeneratingRecipe,
    isParameterModalOpen,
    setIsParameterModalOpen,
    isRecipeWarningModalOpen,
    setIsRecipeWarningModalOpen,
    handleParameterSubmit,
    handleAutoExecution,
    isInitialRecipeLoad,
    hasExistingConversation,
  } = useRecipeManager({
    chat,
    setChat,
    append,
    setView,
    setIsGoosehintsModalOpen,
  });

  // Track recipe usage
  useEffect(() => {
    if (recipeConfig?.title) {
      if (recipeConfig.title !== currentRecipeTitle) {
        setCurrentRecipeTitle(recipeConfig.title);
        setHasStartedUsingRecipe(false);
      } else if (isInitialRecipeLoad) {
        setHasStartedUsingRecipe(false);
      } else if (hasExistingConversation) {
        setHasStartedUsingRecipe(true);
      }
    }
  }, [recipeConfig?.title, currentRecipeTitle, messages.length, setMessages]);

  // Handle recipe auto-execution
  useEffect(() => {
    const isProcessingResponse =
      chatState !== ChatState.Idle && chatState !== ChatState.WaitingForUserInput;
    handleAutoExecution(append, isProcessingResponse, () => {
      setHasStartedUsingRecipe(true);
    });
  }, [handleAutoExecution, append, chatState]);

  // Use shared file drop
  const { droppedFiles, setDroppedFiles, handleDrop, handleDragOver } = useFileDrop();

  // Use shared cost tracking
  const { sessionCosts } = useCostTracking({
    sessionInputTokens,
    sessionOutputTokens,
    sessionMetadata,
  });

  useEffect(() => {
    window.electron.logInfo(
      'Initial messages when resuming session: ' + JSON.stringify(chat.messages, null, 2)
    );
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Auto-scroll when messages are loaded (for session resuming)
  const handleRenderingComplete = React.useCallback(() => {
    if (scrollRef.current?.scrollToBottom) {
      scrollRef.current.scrollToBottom();
    }
  }, []);

  // Handle submit
  const handleSubmit = (e: React.FormEvent) => {
    const customEvent = e as unknown as CustomEvent;
    const combinedTextFromInput = customEvent.detail?.value || '';

    // Mark that user has started using the recipe when they submit a message
    if (recipeConfig && combinedTextFromInput.trim()) {
      setHasStartedUsingRecipe(true);
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

  // Listen for global scroll-to-bottom requests (e.g., from MCP UI prompt actions)
  useEffect(() => {
    const handleGlobalScrollRequest = () => {
      // Add a small delay to ensure content has been rendered
      setTimeout(() => {
        if (scrollRef.current?.scrollToBottom) {
          scrollRef.current.scrollToBottom();
        }
      }, 200);
    };

    window.addEventListener('scroll-chat-to-bottom', handleGlobalScrollRequest);
    return () => window.removeEventListener('scroll-chat-to-bottom', handleGlobalScrollRequest);
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
          <ScrollAreaEnhanced
            ref={scrollRef}
            className={`flex-1 bg-background-default rounded-b-2xl min-h-0 relative ${contentClassName}`}
            autoScroll={!intelligentScrolling} // Use legacy auto-scroll when intelligent scrolling is disabled
            intelligentScroll={intelligentScrolling} // Enable intelligent scrolling
            scrollConfig={scrollConfig} // Pass configuration
            onDrop={handleDrop}
            onDragOver={handleDragOver}
            onMessageClick={(messageId, element) => {
              console.log("🖱️ Message clicked in BaseChat:", messageId);
            }}
            data-drop-zone="true"
            paddingX={6}
            paddingY={0}
          >
            {/* Recipe agent header - sticky at top of chat container */}
            {recipeConfig && (
              <div className="sticky top-0 z-20 bg-background-default border-b border-border-subtle">
                <AgentHeader
                  recipeConfig={recipeConfig}
                  hasStartedUsingRecipe={hasStartedUsingRecipe}
                  onStartRecipe={() => setHasStartedUsingRecipe(true)}
                />
              </div>
            )}

            {/* Content before messages */}
            {renderBeforeMessages && renderBeforeMessages()}

            {/* Main message list */}
            <div className="flex-1 min-h-0">
              <ProgressiveMessageList
                messages={messages}
                isLoading={isLoading}
                onRenderingComplete={handleRenderingComplete}
                disableAnimation={disableAnimation}
                loadingChat={loadingChat}
                chatState={chatState}
                onMessageUpdate={onMessageUpdate}
                sessionMetadata={sessionMetadata}
                sessionCosts={sessionCosts}
                isUserMessage={isUserMessage}
                error={error}
                clearError={clearError}
                stop={stop}
                reload={reload}
                append={appendWithTracking}
                disableSearch={disableSearch}
                showPopularTopics={showPopularTopics}
                suppressEmptyState={suppressEmptyState}
                toolCallNotifications={toolCallNotifications}
                commandHistory={commandHistory}
                onManualCompaction={handleManualCompaction}
                isCompacting={isCompacting}
                loadingMessage={
                  loadingChat
                    ? 'loading conversation...'
                    : isCompacting
                      ? 'goose is compacting the conversation...'
                      : undefined
                }
                chatState={chatState}
              />
            </div>
          </ScrollAreaEnhanced>

          {/* Content after messages */}
          {renderAfterMessages && renderAfterMessages()}

          {/* Debug info for intelligent scrolling (development only) */}
          {intelligentScrolling && process.env.NODE_ENV === 'development' && scrollRef.current && (
            <div className="absolute bottom-20 right-4 z-30 text-xs bg-black/70 text-white px-3 py-2 rounded-lg">
              <div>Intelligent Scrolling: ON</div>
              <div>User State: {scrollRef.current.getUserActivityState?.()}</div>
              <div>Active: {scrollRef.current.isUserActive?.() ? 'Yes' : 'No'}</div>
            </div>
          )}
        </div>

        <div
          className={`relative z-10 ${disableAnimation ? '' : 'animate-[fadein_400ms_ease-in_forwards]'}`}
        >
          <ChatInput
            sessionId={chat.sessionId}
            handleSubmit={handleSubmit}
            input={input}
            setInput={setInput}
            isLoading={isLoading}
            stop={stop}
            append={appendWithTracking}
            droppedFiles={droppedFiles}
            setDroppedFiles={setDroppedFiles}
            autoSubmit={autoSubmit}
            {...customChatInputProps}
          />
        </div>
      </MainPanelLayout>

      {/* Modals */}
      <ParameterInputModal
        isOpen={isParameterModalOpen}
        onClose={() => setIsParameterModalOpen(false)}
        parameters={filteredParameters}
        onSubmit={handleParameterSubmit}
        initialPrompt={initialPrompt}
      />

      <RecipeWarningModal
        isOpen={isRecipeWarningModalOpen}
        onClose={() => setIsRecipeWarningModalOpen(false)}
        onConfirm={() => {
          setIsRecipeWarningModalOpen(false);
          setIsParameterModalOpen(true);
        }}
        recipeTitle={recipeConfig?.title || ''}
      />

      {/* Tool count alert */}
      <ToolCountAlert />
    </div>
  );
}

// Tool count alert component
function ToolCountAlert() {
  const { showAlert, alertMessage, dismissAlert } = useToolCount();

  if (!showAlert) return null;

  return (
    <div className="fixed bottom-4 right-4 z-50 bg-yellow-100 border border-yellow-400 text-yellow-800 px-4 py-3 rounded-lg shadow-lg max-w-md">
      <div className="flex justify-between items-start">
        <div className="text-sm">{alertMessage}</div>
        <button
          onClick={dismissAlert}
          className="ml-2 text-yellow-600 hover:text-yellow-800 font-bold"
        >
          ×
        </button>
      </div>
    </div>
  );
}

// Main component with context provider
export default function BaseChat(props: BaseChatProps) {
  return (
    <ContextManagerProvider>
      <BaseChatContent {...props} />
    </ContextManagerProvider>
  );
}
