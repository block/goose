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

import React, { useEffect, useContext, createContext, useRef, useState, useMemo } from 'react';
import { useLocation } from 'react-router-dom';
import GooseMessage from './GooseMessage';
import UserMessage from './UserMessage';
import { SearchView } from './conversation/SearchView';
import { AgentHeader } from './AgentHeader';
import LayingEggLoader from './LayingEggLoader';
import LoadingGoose from './LoadingGoose';
import Splash from './Splash';
import { SessionSummaryModal } from './context_management/SessionSummaryModal';
import RestoreModal from './RestoreModal';
import { SidecarProvider } from './SidecarLayout';
import {
  ChatContextManagerProvider,
  useChatContextManager,
} from './context_management/ChatContextManager';
import { ContextHandler } from './context_management/ContextHandler';
import { type View, ViewOptions } from '../App';
import { MainPanelLayout } from './Layout/MainPanelLayout';
import ChatInput from './ChatInput';
import { ScrollArea, ScrollAreaHandle } from './ui/scroll-area';
import { useChatEngine } from '../hooks/useChatEngine';
import { useRecipeManager } from '../hooks/useRecipeManager';
import { useSessionContinuation } from '../hooks/useSessionContinuation';
import { useFileDrop } from '../hooks/useFileDrop';
import { useCostTracking } from '../hooks/useCostTracking';
import { useSidecar } from './SidecarLayout';
import { hasDiffContent } from './ToolCallWithResponse';
import { getToolRequests, getToolResponses, Message } from '../types/message';
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
}: BaseChatProps) {
  const location = useLocation();
  const scrollRef = useRef<ScrollAreaHandle>(null);

  // Get disableAnimation from location state
  const disableAnimation = location.state?.disableAnimation || false;

  // State for restore modal
  const [restoreModalFiles, setRestoreModalFiles] = useState<
    { path: string; checkpoint: string; timestamp: string }[] | null
  >(null);

  const {
    summaryContent,
    summarizedThread,
    isSummaryModalOpen,
    isLoadingSummary,
    resetMessagesWithSummary,
    closeSummaryModal,
    updateSummary,
    hasContextHandlerContent,
    getContextHandlerType,
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
    enableLocalStorage,
  });

  // Use shared recipe manager
  const { recipeConfig, initialPrompt, isGeneratingRecipe, handleAutoExecution } = useRecipeManager(
    messages,
    location.state
  );

  // Handle recipe auto-execution
  useEffect(() => {
    handleAutoExecution(append, isLoading);
  }, [handleAutoExecution, append, isLoading]);

  // Use shared session continuation
  useSessionContinuation({
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

  // Get sidecar context to check for collapsed panel
  const sidecar = useSidecar();

  // Check if there are any messages with diff content
  const hasDiffActions = useMemo(() => {
    return filteredMessages.some((message) => {
      const toolRequests = getToolRequests(message);
      if (toolRequests.length === 0) return false;

      // Look for tool responses in subsequent messages
      const messageIndex = messages.findIndex((msg) => msg.id === message.id);
      if (messageIndex === -1) return false;

      for (let i = messageIndex + 1; i < messages.length; i++) {
        const responses = getToolResponses(messages[i]);
        for (const response of responses) {
          const matchingRequest = toolRequests.find((req) => req.id === response.id);
          if (matchingRequest && hasDiffContent(response)) {
            return true;
          }
        }
      }
      return false;
    });
  }, [filteredMessages, messages]);

  const showCollapsedSidecar = hasDiffActions && sidecar;

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

  // Helper function to render messages
  const renderMessages = () => {
    const messagesContent = filteredMessages.map((message, index) => {
      const isUser = isUserMessage(message);

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
                <UserMessage
                  message={message}
                  messages={messages}
                  onRestore={(files) => {
                    // Show the restore modal first
                    setRestoreModalFiles(files);
                  }}
                />
              )}
            </>
          ) : (
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
    });

    // Add collapsed sidecar panel if needed
    if (showCollapsedSidecar) {
      return <div className="relative pl-6 pr-18">{messagesContent}</div>;
    }

    return messagesContent;
  };

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

        {/* Recipe agent header - inside the messages container when present */}
        <div className="flex flex-col flex-1 mb-0.5 min-h-0">
          <ScrollArea
            ref={scrollRef}
            className={`flex-1 bg-background-default rounded-b-2xl min-h-0 ${contentClassName}`}
            autoScroll
            onDrop={handleDrop}
            onDragOver={handleDragOver}
            data-drop-zone="true"
            paddingX={showCollapsedSidecar ? 0 : 6}
            paddingY={0}
          >
            {/* Recipe agent header - now inside the messages container */}
            {recipeConfig?.title && (
              <div className="px-0 -mx-6">
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
                  showBorder={messages.length > 0}
                />
              </div>
            )}

            {/* Custom content before messages */}
            {renderBeforeMessages && renderBeforeMessages()}

            {/* Messages or Splash */}
            {messages.length === 0 ? (
              /* Show Splash when no messages and we have a recipe config */
              recipeConfig && (
                <Splash
                  append={(text: string) => append(text)}
                  activities={
                    Array.isArray(recipeConfig.activities) ? recipeConfig.activities : null
                  }
                  title={recipeConfig.title}
                />
              )
            ) : (
              <>
                {disableSearch ? (
                  // Render messages without SearchView wrapper when search is disabled
                  renderMessages()
                ) : (
                  // Render messages with SearchView wrapper when search is enabled
                  <SearchView>{renderMessages()}</SearchView>
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
            )}

            {/* Loading indicator at bottom of messages container */}
            {isLoading && (
              <div className="px-0 -mx-6">
                <LoadingGoose
                  message={isLoadingSummary ? 'summarizing conversation…' : undefined}
                />
              </div>
            )}

            {/* Custom content after messages */}
            {renderAfterMessages && renderAfterMessages()}
          </ScrollArea>
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

      {/* Restore Modal */}
      {restoreModalFiles && (
        <RestoreModal
          files={restoreModalFiles}
          onConfirm={async (files) => {
            // Create a restore message to send to the chat
            const fileList = files.map((f) => `• ${f.path} (from ${f.timestamp})`).join('\n');
            const restoreMessage = `Please restore the following files to their earlier versions:\n\n${fileList}`;

            // Send the restore message to the chat
            engineHandleSubmit(restoreMessage);

            // Close the modal
            setRestoreModalFiles(null);
          }}
          onClose={() => setRestoreModalFiles(null)}
        />
      )}
    </div>
  );
}

export default function BaseChat(props: BaseChatProps) {
  return (
    <ChatContextManagerProvider>
      <SidecarProvider>
        <BaseChatContent {...props} />
      </SidecarProvider>
    </ChatContextManagerProvider>
  );
}
