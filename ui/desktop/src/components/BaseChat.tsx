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

import React, { createContext, useContext, useEffect, useRef } from 'react';
import { useLocation } from 'react-router-dom';
import { SearchView } from './conversation/SearchView';
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
import { ScrollArea, ScrollAreaHandle } from './ui/scroll-area';
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

// Context for sharing current model info
const CurrentModelContext = createContext<{ model: string; mode: string } | null>(null);
export const useCurrentModelInfo = () => useContext(CurrentModelContext);

interface BaseChatProps {
  chat: ChatType;
  setChat: (chat: ChatType) => void;
  setView: (view: View, viewOptions?: ViewOptions) => void;
  setIsGoosehintsModalOpen?: (isOpen: boolean) => void;
  onMessageStreamFinish?: () => void;
  onMessageSubmit?: (message: string) => void;
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
}

function BaseChatContent({
  chat,
  setChat,
  setView,
  setIsGoosehintsModalOpen,
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
  autoSubmit = false,
  loadingChat = false,
}: BaseChatProps) {
  const location = useLocation();
  const scrollRef = useRef<ScrollAreaHandle>(null);

  const disableAnimation = location.state?.disableAnimation || false;
  const [hasStartedUsingRecipe, setHasStartedUsingRecipe] = React.useState(false);
  const [currentRecipeTitle, setCurrentRecipeTitle] = React.useState<string | null>(null);
  const { isCompacting, handleManualCompaction } = useContextManager();

  // Timeout ref for debouncing auto-scroll
  const autoScrollTimeoutRef = useRef<number | null>(null);
  // Track if user was following when agent started responding
  const isFollowingStreamRef = useRef<boolean>(true);

  const isNearBottom = React.useCallback(() => {
    if (!scrollRef.current) return false;

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const viewport = scrollRef.current as any;
    if (!viewport.viewportRef?.current) return false;

    const viewportElement = viewport.viewportRef.current;
    const { scrollHeight, scrollTop, clientHeight } = viewportElement;
    const scrollBottom = scrollTop + clientHeight;
    const distanceFromBottom = scrollHeight - scrollBottom;

    return distanceFromBottom <= 50;
  }, []);

  // Function to auto-scroll if user was following when agent started
  const conditionalAutoScroll = React.useCallback(() => {
    // Clear any existing timeout
    if (autoScrollTimeoutRef.current) {
      clearTimeout(autoScrollTimeoutRef.current);
    }

    // Debounce the auto-scroll to prevent jumpy behavior and prevent multiple rapid scrolls
    // Immediate scroll if following, with fallback timeout
    if (isFollowingStreamRef.current && scrollRef.current) {
      scrollRef.current.scrollToBottom();
      console.log('📍 Auto-scrolled to follow stream (immediate)');
    }
    
    // Also set a timeout as backup
    autoScrollTimeoutRef.current = window.setTimeout(() => {
      if (isFollowingStreamRef.current && scrollRef.current) {
        scrollRef.current.scrollToBottom();
        console.log('📍 Auto-scrolled to follow stream (timeout backup)');
      }
    }, 50); // Reduced from 150ms to 50ms
  }, []);

  useEffect(() => {
    return () => {
      if (autoScrollTimeoutRef.current) {
        clearTimeout(autoScrollTimeoutRef.current);
      }
    };
  }, []);

  // Setup scroll listener for following behavior (robust version)
  useEffect(() => {
    const viewport = scrollRef.current as any;
    if (!viewport?.viewportRef?.current) return;

    const viewportElement = viewport.viewportRef.current;
    
    const handleScroll = () => {
      // Use a more direct approach to avoid stale closures
      const nearBottom = (() => {
        if (!scrollRef.current) return false;
        const vp = scrollRef.current as any;
        if (!vp.viewportRef?.current) return false;
        const vpElement = vp.viewportRef.current;
        const { scrollHeight, scrollTop, clientHeight } = vpElement;
        const scrollBottom = scrollTop + clientHeight;
        const distanceFromBottom = scrollHeight - scrollBottom;
        return distanceFromBottom <= 50;
      })();
      
      // Update following state based on scroll position
      if (nearBottom && !isFollowingStreamRef.current) {
        // User scrolled back to bottom - resume following
        isFollowingStreamRef.current = true;
        console.log('✅ FOLLOWING STARTED/RESUMED');
        console.log('📍 Resumed following stream - user at bottom');
      } else if (!nearBottom && isFollowingStreamRef.current) {
        // User scrolled away from bottom - stop following
        isFollowingStreamRef.current = false;
        console.log('🛑 FOLLOWING STOPPED - User scrolled away from bottom');
        console.log('📍 Stopped following stream - user scrolled away');
        console.log('🔍 Scroll position debug:', {
          scrollTop: (() => {
            const vp = scrollRef.current as any;
            return vp?.viewportRef?.current?.scrollTop || 'unknown';
          })(),
          scrollHeight: (() => {
            const vp = scrollRef.current as any;
            return vp?.viewportRef?.current?.scrollHeight || 'unknown';
          })(),
          clientHeight: (() => {
            const vp = scrollRef.current as any;
            return vp?.viewportRef?.current?.clientHeight || 'unknown';
          })(),
          distanceFromBottom: (() => {
            const vp = scrollRef.current as any;
            if (!vp?.viewportRef?.current) return 'unknown';
            const { scrollHeight, scrollTop, clientHeight } = vp.viewportRef.current;
            return scrollHeight - (scrollTop + clientHeight);
          })()
        });
      }
    };

    // Add the scroll listener
    viewportElement.addEventListener('scroll', handleScroll, { passive: true });
    
    // Also add focus/blur listeners to handle window focus changes
    const handleFocus = () => {
      console.log('🔍 Window focused - checking scroll position');
      handleScroll(); // Re-check position when window gains focus
    };
    
    const handleBlur = () => {
      console.log('🔍 Window blurred - maintaining following state');
      // Don't change following state on blur, just log it
    };
    
    window.addEventListener('focus', handleFocus);
    window.addEventListener('blur', handleBlur);

    return () => {
      viewportElement.removeEventListener('scroll', handleScroll);
      window.removeEventListener('focus', handleFocus);
      window.removeEventListener('blur', handleBlur);
    };
  }, []); // Remove handleScrollChange dependency to avoid re-creating listener

  // More frequent check to ensure following state stays correct
  useEffect(() => {
    const interval = setInterval(() => {
      if (scrollRef.current) {
        const nearBottom = isNearBottom();
        if (nearBottom && !isFollowingStreamRef.current) {
          isFollowingStreamRef.current = true;
        console.log('✅ FOLLOWING STARTED/RESUMED');
          console.log('🔄 Periodic check: Resumed following stream');
          // Immediately scroll if we just resumed following
          scrollRef.current.scrollToBottom();
        }
      }
    }, 500); // Check every 500ms instead of 2 seconds

    return () => clearInterval(interval);
  }, [isNearBottom]);

  // Use shared chat engine
  const {
    messages,
    filteredMessages,
    append,
    chatState,
    error,
    setMessages,
    input,
    handleSubmit: engineHandleSubmit,
    onStopGoose,
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
      console.log('🏁 Stream finished - calling conditionalAutoScroll');
      conditionalAutoScroll();

      // Call the original callback if provided
      onMessageStreamFinish?.();
    },
    onMessageStreamStart: () => {
      // When streaming starts, check if we should follow
      isFollowingStreamRef.current = isNearBottom();
      console.log('🚀 Stream started - following:', isFollowingStreamRef.current);
      
      onMessageStreamStart?.();
    },
    onMessageSent: () => {
      isFollowingStreamRef.current = true;
        console.log('✅ FOLLOWING STARTED/RESUMED');
      console.log('📤 Message sent - will follow response');
      // Ensure we scroll to bottom immediately when sending
      setTimeout(() => {
        if (scrollRef.current) {
          scrollRef.current.scrollToBottom();
        }
      }, 100);

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
    recipeParameters,
    handleParameterSubmit,
    handleAutoExecution,
    recipeError,
    setRecipeError,
    isRecipeWarningModalOpen,
    recipeAccepted,
    handleRecipeAccept,
    handleRecipeCancel,
    hasSecurityWarnings,
  } = useRecipeManager(chat, location.state?.recipeConfig);

  // Auto-scroll when messages change during streaming - ENHANCED DEBUG
  useEffect(() => {
    console.log('🔍 Messages useEffect triggered:', {
      messagesLength: messages.length,
      isFollowing: isFollowingStreamRef.current,
      hasScrollRef: !!scrollRef.current
    });
    
    if (messages.length > 0) {
      if (isFollowingStreamRef.current) {
        console.log('📨 Messages updated - following stream, scrolling to bottom');
        if (scrollRef.current) {
          console.log('🚀 Calling scrollToBottom...');
          scrollRef.current.scrollToBottom();
          console.log('✅ scrollToBottom called successfully');
        } else {
          console.log('❌ scrollRef.current is null!');
        }
      } else {
        console.log('⏸️ Messages updated but not following - isFollowing:', isFollowingStreamRef.current);
      }
    } else {
      console.log('📭 No messages yet');
    }
  }, [messages]);
  // Reset recipe usage tracking when recipe changes
  useEffect(() => {
    const previousTitle = currentRecipeTitle;
    const newTitle = recipeConfig?.title || null;
    const hasRecipeChanged = newTitle !== currentRecipeTitle;

    if (hasRecipeChanged) {
      setCurrentRecipeTitle(newTitle);

      const isSwitchingBetweenRecipes = previousTitle && newTitle;
      const isInitialRecipeLoad = !previousTitle && newTitle && messages.length === 0;
      const hasExistingConversation = newTitle && messages.length > 0;

      if (isSwitchingBetweenRecipes) {
        console.log('Switching from recipe:', previousTitle, 'to:', newTitle);
        setHasStartedUsingRecipe(false);
        setMessages([]);
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
    localInputTokens,
    localOutputTokens,
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

    // Call the callback if provided (for Hub to handle navigation)
    if (onMessageSubmit && combinedTextFromInput.trim()) {
      onMessageSubmit(combinedTextFromInput);
    }

    engineHandleSubmit(combinedTextFromInput);
  };

  const toolCount = useToolCount(chat.sessionId);

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
          <ScrollArea
            ref={scrollRef}
            className={`flex-1 bg-background-default rounded-b-2xl min-h-0 relative ${contentClassName}`}
            autoScroll={false}
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

            {/* Recipe Activities - always show when recipe is active and accepted */}
            {recipeConfig && recipeAccepted && !suppressEmptyState && (
              <div className={hasStartedUsingRecipe ? 'mb-6' : ''}>
                <RecipeActivities
                  append={(text: string) => appendWithTracking(text)}
                  activities={
                    Array.isArray(recipeConfig.activities) ? recipeConfig.activities : null
                  }
                  title={recipeConfig.title}
                  parameterValues={recipeParameters || {}}
                />
              </div>
            )}

            {/* Messages or Popular Topics */}
            {
              loadingChat ? null : filteredMessages.length > 0 ||
                (recipeConfig && recipeAccepted && hasStartedUsingRecipe) ? (
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
                      isStreamingMessage={chatState !== ChatState.Idle}
                      onMessageUpdate={onMessageUpdate}
                      onRenderingComplete={handleRenderingComplete}
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
                        isStreamingMessage={chatState !== ChatState.Idle}
                        onMessageUpdate={onMessageUpdate}
                        onRenderingComplete={handleRenderingComplete}
                      />
                    </SearchView>
                  )}

                  {error && (
                    <>
                      <div className="flex flex-col items-center justify-center p-4">
                        <div className="text-red-700 dark:text-red-300 bg-red-400/50 p-3 rounded-lg mb-2">
                          {error.message || 'Honk! Goose experienced an error while responding'}
                        </div>

                        {/* Action buttons for all errors including token limit errors */}
                        <div className="flex gap-2 mt-2">
                          <div
                            className="px-3 py-2 text-center whitespace-nowrap cursor-pointer text-textStandard border border-borderSubtle hover:bg-bgSubtle rounded-full inline-block transition-all duration-150"
                            onClick={async () => {
                              clearError();

                              await handleManualCompaction(messages, setMessages, append);
                            }}
                          >
                            Summarize Conversation
                          </div>
                          <div
                            className="px-3 py-2 text-center whitespace-nowrap cursor-pointer text-textStandard border border-borderSubtle hover:bg-bgSubtle rounded-full inline-block transition-all duration-150"
                            onClick={async () => {
                              // Find the last user message
                              const lastUserMessage = messages.reduceRight(
                                (found, m) => found || (m.role === 'user' ? m : null),
                                null as Message | null
                              );
                              if (lastUserMessage) {
                                await append(lastUserMessage);
                              }
                            }}
                          >
                            Retry Last Message
                          </div>
                        </div>
                      </div>
                    </>
                  )}

                  <div className="block h-8" />
                </>
              ) : !recipeConfig && showPopularTopics ? (
                /* Show PopularChatTopics when no messages, no recipe, and showPopularTopics is true (Pair view) */
                <PopularChatTopics append={(text: string) => append(text)} />
              ) : null /* Show nothing when messages.length === 0 && suppressEmptyState === true */
            }

            {/* Custom content after messages */}
            {renderAfterMessages && renderAfterMessages()}
          </ScrollArea>

          {/* Fixed loading indicator at bottom left of chat container */}
          {(chatState !== ChatState.Idle || loadingChat || isCompacting) && (
            <div className="absolute bottom-1 left-4 z-20 pointer-events-none">
              <LoadingGoose
                message={
                  loadingChat
                    ? 'loading conversation...'
                    : isCompacting
                      ? 'goose is compacting the conversation...'
                      : undefined
                }
                chatState={chatState}
              />
            </div>
          )}
        </div>

        <div
          className={`relative z-10 ${disableAnimation ? '' : 'animate-[fadein_400ms_ease-in_forwards]'}`}
        >
          <ChatInput
            sessionId={chat.sessionId}
            handleSubmit={handleSubmit}
            chatState={chatState}
            onStop={onStopGoose}
            commandHistory={commandHistory}
            initialValue={input || ''}
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
            recipeAccepted={recipeAccepted}
            initialPrompt={initialPrompt}
            toolCount={toolCount || 0}
            autoSubmit={autoSubmit}
            append={append}
            {...customChatInputProps}
          />
        </div>
      </MainPanelLayout>

      {/* Recipe Warning Modal */}
      <RecipeWarningModal
        isOpen={isRecipeWarningModalOpen}
        onConfirm={handleRecipeAccept}
        onCancel={handleRecipeCancel}
        recipeDetails={{
          title: recipeConfig?.title,
          description: recipeConfig?.description,
          instructions: recipeConfig?.instructions || undefined,
        }}
        hasSecurityWarnings={hasSecurityWarnings}
      />

      {/* Recipe Parameter Modal */}
      {isParameterModalOpen && filteredParameters.length > 0 && (
        <ParameterInputModal
          parameters={filteredParameters}
          onSubmit={handleParameterSubmit}
          onClose={() => setIsParameterModalOpen(false)}
        />
      )}

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

      {/* No modals needed for the new simplified context manager */}
    </div>
  );
}

export default function BaseChat(props: BaseChatProps) {
  return (
    <ContextManagerProvider>
      <BaseChatContent {...props} />
    </ContextManagerProvider>
  );
}
