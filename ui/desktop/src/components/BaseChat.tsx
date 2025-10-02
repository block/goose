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

import React, { createContext, useContext, useEffect, useRef, useState } from 'react';
import { useLocation } from 'react-router-dom';
import { SearchView } from './conversation/SearchView';
import { AgentHeader } from './AgentHeader';
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
import CreateRecipeFromSessionModal from './recipes/CreateRecipeFromSessionModal';
import { useChatEngine } from '../hooks/useChatEngine';
import { useRecipeState } from '../hooks/useRecipeState';
import { updateSystemPromptWithParameters, substituteParameters } from '../utils/providerUtils';
import { updateSessionUserRecipeValues } from '../api';
import { createUserMessage } from '../types/message';
import { Recipe } from '../recipe';
import { useRecipeCreationModal } from '../hooks/useRecipeCreationModal';
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
  onMessageSubmit?: () => void;
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
      // Call the original callback if provided
      onMessageStreamFinish?.();
    },
    onMessageSent: () => {
      // Mark that user has started using the recipe
      if (recipeConfig) {
        setHasStartedUsingRecipe(true);
      }
    },
  });

  // Use recipe state - no complex heuristics
  const recipeState = useRecipeState(location.state?.recipeConfig || chat.recipeConfig);

  // Local UI state for modals
  const [isParameterModalOpen, setIsParameterModalOpen] = useState(false);
  const [isRecipeWarningModalOpen, setIsRecipeWarningModalOpen] = useState(false);
  const [readyForAutoUserPrompt, setReadyForAutoUserPrompt] = useState(false);

  // Computed values using the simple approach
  const recipeConfig = recipeState.recipe;
  const recipeParameters = chat.recipeParameters;
  const filteredParameters = recipeState.filteredParameters;
  const hasAllRequiredParameters = recipeState.hasAllRequiredParameters(recipeParameters || null);
  const initialPrompt = recipeState.getInitialPrompt(recipeParameters || null);
  const recipeAccepted = recipeState.recipeAccepted;
  const hasSecurityWarnings = recipeState.hasSecurityWarnings;

  // Use recipe creation modal hook for UI-specific functionality
  const { isCreateRecipeModalOpen, setIsCreateRecipeModalOpen } = useRecipeCreationModal(
    chat.sessionId
  );

  // Handle parameter modal display logic
  useEffect(() => {
    // Only show parameter modal if:
    // 1. Recipe requires parameters
    // 2. Recipe has been accepted
    // 3. Not all required parameters have been filled in yet
    // 4. Parameter modal is not already open (prevent multiple opens)
    // 5. No messages in chat yet (don't show after conversation has started)
    if (
      recipeState.requiresParameters &&
      recipeAccepted &&
      !hasAllRequiredParameters &&
      !isParameterModalOpen &&
      messages.length === 0
    ) {
      setIsParameterModalOpen(true);
    }
  }, [
    recipeState.requiresParameters,
    hasAllRequiredParameters,
    recipeAccepted,
    isParameterModalOpen,
    messages.length,
    chat.sessionId,
    recipeConfig?.title,
  ]);

  // Handle recipe warning modal display
  useEffect(() => {
    if (recipeConfig && !recipeAccepted && hasSecurityWarnings) {
      setIsRecipeWarningModalOpen(true);
    }
  }, [recipeConfig, recipeAccepted, hasSecurityWarnings]);

  useEffect(() => {
    setReadyForAutoUserPrompt(true);
  }, []);

  // UI interaction handlers
  const handleParameterSubmit = async (inputValues: Record<string, string>) => {
    // Update chat state with parameters
    setChat({
      ...chat,
      recipeParameters: inputValues,
    });
    setIsParameterModalOpen(false);

    try {
      await updateSystemPromptWithParameters(
        chat.sessionId,
        inputValues,
        recipeConfig || undefined
      );
      // Update session user recipe values via API
      await updateSessionUserRecipeValues({
        path: { session_id: chat.sessionId },
        body: { userRecipeValues: inputValues },
      });
    } catch (error) {
      console.error('Failed to update system prompt with parameters:', error);
    }
  };

  const handleRecipeAccept = async () => {
    await recipeState.acceptRecipe();
    setIsRecipeWarningModalOpen(false);
  };

  const handleRecipeCancel = () => {
    setIsRecipeWarningModalOpen(false);
    window.electron.closeWindow();
  };

  const handleAutoExecution = React.useCallback(
    (append: (message: Message) => void, isLoading: boolean, onAutoExecute?: () => void) => {
      if (
        recipeConfig?.isScheduledExecution &&
        recipeConfig?.prompt &&
        (!recipeState.requiresParameters || recipeParameters) &&
        messages.length === 0 &&
        !isLoading &&
        readyForAutoUserPrompt &&
        recipeAccepted
      ) {
        const finalPrompt = recipeParameters
          ? substituteParameters(recipeConfig.prompt, recipeParameters)
          : recipeConfig.prompt;

        const userMessage = createUserMessage(finalPrompt);
        append(userMessage);
        onAutoExecute?.();
      }
    },
    [
      recipeConfig?.isScheduledExecution,
      recipeConfig?.prompt,
      recipeState.requiresParameters,
      recipeParameters,
      messages.length,
      readyForAutoUserPrompt,
      recipeAccepted,
    ]
  );

  const handleRecipeCreated = (recipe: Recipe, onRecipeCreated?: (recipe: Recipe) => void) => {
    // Delegate toast/notification responsibility to the caller
    onRecipeCreated?.(recipe);
  };

  const handleStartRecipe = (recipe: Recipe) => {
    setChat({
      ...chat,
      messages: [],
      recipeConfig: recipe,
      recipeParameters: null,
    });
  };

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
        setHasStartedUsingRecipe(false);
      } else if (isInitialRecipeLoad) {
        setHasStartedUsingRecipe(false);
      } else if (hasExistingConversation) {
        setHasStartedUsingRecipe(true);
      }
    }
  }, [recipeConfig?.title, currentRecipeTitle, messages.length]);

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
    session: sessionMetadata,
  });

  useEffect(() => {
    window.electron.logInfo(
      'Initial messages when resuming session: ' + JSON.stringify(chat.messages, null, 2)
    );
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Track if this is the initial render for session resuming
  const initialRenderRef = useRef(true);

  // Auto-scroll when messages are loaded (for session resuming)
  const handleRenderingComplete = React.useCallback(() => {
    // Only force scroll on the very first render
    if (initialRenderRef.current && messages.length > 0) {
      initialRenderRef.current = false;
      if (scrollRef.current?.scrollToBottom) {
        scrollRef.current.scrollToBottom();
      }
    } else if (scrollRef.current?.isFollowing) {
      if (scrollRef.current?.scrollToBottom) {
        scrollRef.current.scrollToBottom();
      }
    }
  }, [messages.length]);

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
      onMessageSubmit();
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
              loadingChat ? null : filteredMessages.length > 0 ? (
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

                              await handleManualCompaction(
                                messages,
                                setMessages,
                                append,
                                chat.sessionId
                              );
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
        onCancel={() => handleRecipeCancel()}
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

      {/* Create Recipe from Session Modal */}
      <CreateRecipeFromSessionModal
        isOpen={isCreateRecipeModalOpen}
        onClose={() => setIsCreateRecipeModalOpen(false)}
        sessionId={chat.sessionId}
        onRecipeCreated={(recipe) =>
          handleRecipeCreated(recipe, (recipe) => {
            // Handle toast notification at the component level
            import('../toasts').then(({ toastSuccess }) => {
              toastSuccess({
                title: 'Recipe created successfully!',
                msg: `"${recipe.title}" has been saved and is ready to use.`,
              });
            });
          })
        }
        onStartRecipe={handleStartRecipe}
      />
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
