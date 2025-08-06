/**
 * ProgressiveMessageList Component
 *
 * A performance-optimized message list that renders messages progressively
 * to prevent UI blocking when loading long chat sessions. This component
 * renders messages in batches with a loading indicator, maintaining full
 * compatibility with the search functionality.
 *
 * Key Features:
 * - Progressive rendering in configurable batches
 * - Loading indicator during batch processing
 * - Maintains search functionality compatibility
 * - Smooth user experience with responsive UI
 * - Configurable batch size and delay
 */

import { useState, useEffect, useCallback, useRef } from 'react';
import { Message } from '../types/message';
import GooseMessage from './GooseMessage';
import UserMessage from './UserMessage';
import { ContextHandler } from './context_management/ContextHandler';
import { useChatContextManager } from './context_management/ChatContextManager';
import { NotificationEvent } from '../hooks/useMessageStream';
import LoadingGoose from './LoadingGoose';

interface ProgressiveMessageListProps {
  messages: Message[];
  chat?: { id: string; messageHistoryIndex: number }; // Make optional for session history
  toolCallNotifications?: Map<string, NotificationEvent[]>; // Make optional
  append?: (value: string) => void; // Make optional
  appendMessage?: (message: Message) => void; // Make optional
  isUserMessage: (message: Message) => boolean;
  onScrollToBottom?: () => void;
  batchSize?: number;
  batchDelay?: number;
  showLoadingThreshold?: number; // Only show loading if more than X messages
  // Custom render function for messages
  renderMessage?: (message: Message, index: number) => React.ReactNode | null;
  isStreamingMessage?: boolean; // Whether messages are currently being streamed
}

export default function ProgressiveMessageList({
  messages,
  chat,
  toolCallNotifications = new Map(),
  append = () => {},
  appendMessage = () => {},
  isUserMessage,
  onScrollToBottom,
  batchSize = 20,
  batchDelay = 20,
  showLoadingThreshold = 50,
  renderMessage, // Custom render function
  isStreamingMessage = false, // Whether messages are currently being streamed
}: ProgressiveMessageListProps) {
  const [renderedCount, setRenderedCount] = useState(() => {
    // Initialize with either all messages (if small) or first batch (if large)
    return messages.length <= showLoadingThreshold
      ? messages.length
      : Math.min(batchSize, messages.length);
  });
  const [isLoading, setIsLoading] = useState(() => messages.length > showLoadingThreshold);
  const timeoutRef = useRef<number | null>(null);
  const mountedRef = useRef(true);
  const hasOnlyToolResponses = (message: Message) =>
    message.content.every((c) => c.type === 'toolResponse');

  // Try to use context manager, but don't require it for session history
  let hasContextHandlerContent: ((message: Message) => boolean) | undefined;
  let getContextHandlerType:
    | ((message: Message) => 'contextLengthExceeded' | 'summarizationRequested')
    | undefined;

  try {
    const contextManager = useChatContextManager();
    hasContextHandlerContent = contextManager.hasContextHandlerContent;
    getContextHandlerType = contextManager.getContextHandlerType;
  } catch (error) {
    // Context manager not available (e.g., in session history view)
    // This is fine, we'll just skip context handler functionality
    hasContextHandlerContent = undefined;
    getContextHandlerType = undefined;
  }

  // Simple progressive loading - start immediately when component mounts if needed
  useEffect(() => {
    if (messages.length <= showLoadingThreshold) {
      setRenderedCount(messages.length);
      setIsLoading(false);
      return;
    }

    // Large list - start progressive loading
    const loadNextBatch = () => {
      setRenderedCount((current) => {
        const nextCount = Math.min(current + batchSize, messages.length);

        if (nextCount >= messages.length) {
          setIsLoading(false);
          // Trigger scroll to bottom
          window.setTimeout(() => {
            onScrollToBottom?.();
          }, 100);
        } else {
          // Schedule next batch
          timeoutRef.current = window.setTimeout(loadNextBatch, batchDelay);
        }

        return nextCount;
      });
    };

    // Start loading after a short delay
    timeoutRef.current = window.setTimeout(loadNextBatch, batchDelay);

    return () => {
      if (timeoutRef.current) {
        window.clearTimeout(timeoutRef.current);
        timeoutRef.current = null;
      }
    };
  }, [
    messages.length,
    batchSize,
    batchDelay,
    showLoadingThreshold,
    onScrollToBottom,
    renderedCount,
  ]);

  // Cleanup on unmount
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      if (timeoutRef.current) {
        window.clearTimeout(timeoutRef.current);
      }
    };
  }, []);

  // Force complete rendering when search is active
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const isMac = window.electron.platform === 'darwin';
      const isSearchShortcut = (isMac ? e.metaKey : e.ctrlKey) && e.key === 'f';

      if (isSearchShortcut && isLoading) {
        // Immediately render all messages when search is triggered
        setRenderedCount(messages.length);
        setIsLoading(false);
        if (timeoutRef.current) {
          window.clearTimeout(timeoutRef.current);
          timeoutRef.current = null;
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isLoading, messages.length]);

  // Render messages up to the current rendered count
  const renderMessages = useCallback(() => {
    const messagesToRender = messages.slice(0, renderedCount);

    const renderedMessages = messagesToRender
      .map((message, index) => {
        // Use custom render function if provided
        if (renderMessage) {
          return renderMessage(message, index);
        }

        // Default rendering logic (for BaseChat)
        if (!chat) {
          console.warn(
            'ProgressiveMessageList: chat prop is required when not using custom renderMessage'
          );
          return null;
        }

        const isUser = isUserMessage(message);

        const result = (
          <div
            key={message.id && `${message.id}-${message.content.length}`}
            className={`relative ${index === 0 ? 'mt-0' : 'mt-4'} ${isUser ? 'user' : 'assistant'}`}
            data-testid="message-container"
          >
            {isUser ? (
              <>
                {hasContextHandlerContent && hasContextHandlerContent(message) ? (
                  <ContextHandler
                    messages={messages}
                    messageId={message.id ?? message.created.toString()}
                    chatId={chat.id}
                    workingDir={window.appConfig.get('GOOSE_WORKING_DIR') as string}
                    contextType={getContextHandlerType!(message)}
                    onSummaryComplete={() => {
                      window.setTimeout(() => onScrollToBottom?.(), 100);
                    }}
                  />
                ) : (
                  !hasOnlyToolResponses(message) && <UserMessage message={message} />
                )}
              </>
            ) : (
              <>
                {hasContextHandlerContent && hasContextHandlerContent(message) ? (
                  <ContextHandler
                    messages={messages}
                    messageId={message.id ?? message.created.toString()}
                    chatId={chat.id}
                    workingDir={window.appConfig.get('GOOSE_WORKING_DIR') as string}
                    contextType={getContextHandlerType!(message)}
                    onSummaryComplete={() => {
                      window.setTimeout(() => onScrollToBottom?.(), 100);
                    }}
                  />
                ) : (
                  <GooseMessage
                    messageHistoryIndex={chat.messageHistoryIndex}
                    message={message}
                    messages={messages}
                    append={append}
                    appendMessage={appendMessage}
                    toolCallNotifications={toolCallNotifications}
                    isStreaming={
                      isStreamingMessage &&
                      !isUser &&
                      index === messagesToRender.length - 1 &&
                      message.role === 'assistant'
                    }
                  />
                )}
              </>
            )}
          </div>
        );

        return result;
      })
      .filter(Boolean); // Filter out null values

    return renderedMessages;
  }, [
    messages,
    renderedCount,
    renderMessage,
    isUserMessage,
    hasContextHandlerContent,
    getContextHandlerType,
    chat,
    append,
    appendMessage,
    toolCallNotifications,
    onScrollToBottom,
    isStreamingMessage,
  ]);

  return (
    <>
      {renderMessages()}

      {/* Loading indicator when progressively rendering */}
      {isLoading && (
        <div className="flex flex-col items-center justify-center py-8">
          <LoadingGoose message={`Loading messages... (${renderedCount}/${messages.length})`} />
          <div className="text-xs text-text-muted mt-2">
            Press Cmd/Ctrl+F to load all messages immediately for search
          </div>
        </div>
      )}
    </>
  );
}
