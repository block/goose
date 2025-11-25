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

import { useCallback, useEffect, useRef, useState } from 'react';
import { Message } from '../types/message';
import GooseMessage from './GooseMessage';
import UserMessage from './UserMessage';
import { CompactionMarker } from './context_management/CompactionMarker';
import { useContextManager } from './context_management/ContextManager';
import { NotificationEvent } from '../hooks/useMessageStream';
import LoadingGoose from './LoadingGoose';
import { ChatType } from '../types/chat';
import { MessageComment, TextSelection } from '../types/comment';

interface ProgressiveMessageListProps {
  messages: Message[];
  chat?: Pick<ChatType, 'sessionId' | 'messageHistoryIndex'>;
  toolCallNotifications?: Map<string, NotificationEvent[]>; // Make optional
  append?: (value: string) => void; // Make optional
  appendMessage?: (message: Message) => void; // Make optional
  isUserMessage: (message: Message) => boolean;
  batchSize?: number;
  batchDelay?: number;
  showLoadingThreshold?: number; // Only show loading if more than X messages
  // Custom render function for messages
  renderMessage?: (message: Message, index: number) => React.ReactNode | null;
  isStreamingMessage?: boolean; // Whether messages are currently being streamed
  onMessageUpdate?: (messageId: string, newContent: string) => void;
  onRenderingComplete?: () => void; // Callback when all messages are rendered
  tabId?: string; // Tab ID for opening sidecars
  // Comment-related props
  comments?: Map<string, MessageComment[]>;
  activeSelection?: TextSelection | null;
  activePosition?: { x: number; y: number } | null;
  activeMessageId?: string | null;
  isCreatingComment?: boolean;
  onSelectionChange?: (selection: TextSelection | null, position?: { x: number; y: number }, messageId?: string) => void;
  onCreateComment?: (messageId: string, selection: TextSelection, content: string) => void;
  onUpdateComment?: (commentId: string, content: string) => void;
  onDeleteComment?: (commentId: string) => void;
  onReplyToComment?: (parentId: string, content: string) => void;
  onResolveComment?: (commentId: string, resolved: boolean) => void;
  onCancelComment?: () => void;
  onFocusComment?: (commentId: string) => void;
}

export default function ProgressiveMessageList({
  messages,
  chat,
  toolCallNotifications = new Map(),
  append = () => {},
  appendMessage = () => {},
  isUserMessage,
  batchSize = 20,
  batchDelay = 20,
  showLoadingThreshold = 50,
  renderMessage, // Custom render function
  isStreamingMessage = false, // Whether messages are currently being streamed
  onMessageUpdate,
  onRenderingComplete,
  tabId,
  // Comment props
  comments = new Map(),
  activeSelection,
  activePosition,
  activeMessageId,
  isCreatingComment = false,
  onSelectionChange,
  onCreateComment,
  onUpdateComment,
  onDeleteComment,
  onReplyToComment,
  onResolveComment,
  onCancelComment,
  onFocusComment,
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
  let hasCompactionMarker: ((message: Message) => boolean) | undefined;

  try {
    const contextManager = useContextManager();
    hasCompactionMarker = contextManager.hasCompactionMarker;
  } catch {
    // Context manager not available (e.g., in session history view)
    // This is fine, we'll just skip compaction marker functionality
    hasCompactionMarker = undefined;
  }

  // Simple progressive loading - start immediately when component mounts if needed
  useEffect(() => {
    if (messages.length <= showLoadingThreshold) {
      setRenderedCount(messages.length);
      setIsLoading(false);
      // For small lists, call completion callback immediately
      if (onRenderingComplete) {
        setTimeout(() => onRenderingComplete(), 50);
      }
      return;
    }

    // Large list - start progressive loading
    const loadNextBatch = () => {
      setRenderedCount((current) => {
        const nextCount = Math.min(current + batchSize, messages.length);

        if (nextCount >= messages.length) {
          setIsLoading(false);
          // Call the completion callback after a brief delay to ensure DOM is updated
          if (onRenderingComplete) {
            setTimeout(() => onRenderingComplete(), 50);
          }
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
    renderedCount,
    onRenderingComplete,
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
    // Only add listener if we're actually loading
    if (!isLoading) {
      return;
    }

    const handleKeyDown = (e: KeyboardEvent) => {
      const isMac = window.electron.platform === 'darwin';
      const isSearchShortcut = (isMac ? e.metaKey : e.ctrlKey) && e.key === 'f';

      if (isSearchShortcut) {
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
    return messagesToRender
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

        return (
          <div
            key={message.id && `${message.id}-${message.content.length}`}
            className={`relative ${index === 0 ? 'mt-0' : 'mt-4'} ${isUser ? 'user' : 'assistant'}`}
            data-testid="message-container"
          >
            {isUser ? (
              <>
                {hasCompactionMarker && hasCompactionMarker(message) ? (
                  <CompactionMarker message={message} />
                ) : (
                  !hasOnlyToolResponses(message) && (
                    <UserMessage message={message} onMessageUpdate={onMessageUpdate} />
                  )
                )}
              </>
            ) : (
              <>
                {hasCompactionMarker && hasCompactionMarker(message) ? (
                  <CompactionMarker message={message} />
                ) : (
                  <GooseMessage
                    sessionId={chat.sessionId}
                    messageHistoryIndex={chat.messageHistoryIndex}
                    message={message}
                    messages={messages}
                    messageIndex={index} // Pass the correct sequential message index
                    append={append}
                    appendMessage={appendMessage}
                    toolCallNotifications={toolCallNotifications}
                    tabId={tabId}
                    isStreaming={
                      isStreamingMessage &&
                      !isUser &&
                      index === messagesToRender.length - 1 &&
                      message.role === 'assistant'
                    }
                    // Comment props - use same unique ID logic as GooseMessage
                    comments={(() => {
                      // Generate the same unique ID that GooseMessage uses
                      const uniqueMessageId = message.id || (() => {
                        const contentHash = message.content
                          .map(c => c.type === 'text' ? c.text.slice(0, 50) : c.type)
                          .join('|');
                        return `${message.role}-${message.created}-${index}-${contentHash.length}`;
                      })();
                      return comments.get(uniqueMessageId) || [];
                    })()}
                    activeSelection={activeSelection}
                    activePosition={activePosition}
                    activeMessageId={activeMessageId}
                    isCreatingComment={isCreatingComment}
                    onSelectionChange={onSelectionChange}
                    onCreateComment={onCreateComment}
                    onUpdateComment={onUpdateComment}
                    onDeleteComment={onDeleteComment}
                    onReplyToComment={onReplyToComment}
                    onResolveComment={onResolveComment}
                    onCancelComment={onCancelComment}
                    onFocusComment={onFocusComment}
                  />
                )}
              </>
            )}
          </div>
        );
      })
      .filter(Boolean);
  }, [
    messages,
    renderedCount,
    renderMessage,
    isUserMessage,
    chat,
    append,
    appendMessage,
    toolCallNotifications,
    isStreamingMessage,
    onMessageUpdate,
    hasCompactionMarker,
    // Comment dependencies
    comments,
    activeSelection,
    activePosition,
    activeMessageId,
    isCreatingComment,
    onSelectionChange,
    onCreateComment,
    onUpdateComment,
    onDeleteComment,
    onReplyToComment,
    onResolveComment,
    onCancelComment,
    onFocusComment,
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
