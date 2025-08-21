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
import SystemAlertMessage from './SystemAlertMessage';

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
  systemAlerts?: Array<{ message: string; level: string; timestamp: number }>; // System alerts to display inline
  onMessageUpdate?: (messageId: string, newContent: string) => void;
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
  systemAlerts = [], // System alerts to display inline
  onMessageUpdate,
}: ProgressiveMessageListProps) {
  const [renderedCount, setRenderedCount] = useState(() => {
    // Initialize with either all messages (if small) or first batch (if large)
    return messages.length <= showLoadingThreshold
      ? messages.length
      : Math.min(batchSize, messages.length);
  });
  const [isLoading, setIsLoading] = useState(() => messages.length > showLoadingThreshold);
  const [displayedAlerts, setDisplayedAlerts] = useState<Set<number>>(new Set());
  const timeoutRef = useRef<number | null>(null);
  const mountedRef = useRef(true);
  const hasOnlyToolResponses = useCallback(
    (message: Message) => message.content.every((c) => c.type === 'toolResponse'),
    []
  );

  // Try to use context manager, but don't require it for session history
  let hasContextHandlerContent: ((message: Message) => boolean) | undefined;
  let getContextHandlerType:
    | ((message: Message) => 'contextLengthExceeded' | 'summarizationRequested')
    | undefined;

  try {
    const contextManager = useChatContextManager();
    hasContextHandlerContent = contextManager.hasContextHandlerContent;
    getContextHandlerType = contextManager.getContextHandlerType;
  } catch {
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

  // Track which alerts have been displayed to avoid duplicates and trigger scroll
  useEffect(() => {
    if (systemAlerts.length > 0) {
      const newAlerts = systemAlerts.filter((alert) => !displayedAlerts.has(alert.timestamp));
      if (newAlerts.length > 0) {
        setDisplayedAlerts((prev) => {
          const updated = new Set(prev);
          newAlerts.forEach((alert) => updated.add(alert.timestamp));
          return updated;
        });

        // Ensure scroll happens after DOM updates and animations
        // Use multiple techniques to ensure reliable scrolling
        const scrollToBottom = () => {
          onScrollToBottom?.();
          // Also try to scroll the last alert into view
          requestAnimationFrame(() => {
            const alerts = document.querySelectorAll('[data-testid="system-alert-message"]');
            if (alerts.length > 0) {
              const lastAlert = alerts[alerts.length - 1];
              lastAlert.scrollIntoView({ behavior: 'smooth', block: 'end' });
            }
          });
        };

        // Initial scroll attempt
        requestAnimationFrame(() => {
          setTimeout(scrollToBottom, 50);
        });

        // Backup scroll attempt after animation completes
        setTimeout(scrollToBottom, 350);
      }
    }
  }, [systemAlerts, displayedAlerts, onScrollToBottom]);

  // Render messages up to the current rendered count
  const renderMessages = useCallback(() => {
    const messagesToRender = messages.slice(0, renderedCount);
    const renderedElements: React.ReactNode[] = [];

    // Add system alerts that should appear before the first message
    const alertsBeforeMessages = systemAlerts.filter((alert) => {
      // Show alerts that came before the first message or if there are no messages
      return (
        messagesToRender.length === 0 ||
        (messagesToRender[0].created && alert.timestamp < messagesToRender[0].created * 1000)
      );
    });

    alertsBeforeMessages.forEach((alert, idx) => {
      renderedElements.push(
        <div key={`alert-before-${alert.timestamp}-${idx}`} className="mt-4">
          <SystemAlertMessage
            message={alert.message}
            level={alert.level as 'info' | 'warning' | 'error' | 'success'}
            timestamp={alert.timestamp}
          />
        </div>
      );
    });

    messagesToRender.forEach((message, index) => {
      // Check for alerts that should appear between this message and the next
      const nextMessage = messagesToRender[index + 1];
      const alertsBetween = systemAlerts.filter((alert) => {
        const afterCurrent = alert.timestamp >= message.created * 1000;
        const beforeNext = !nextMessage || alert.timestamp < nextMessage.created * 1000;
        return afterCurrent && beforeNext;
      });

      // Use custom render function if provided
      if (renderMessage) {
        const rendered = renderMessage(message, index);
        if (rendered) {
          renderedElements.push(rendered);
        }
      } else {
        // Default rendering logic (for BaseChat)
        if (!chat) {
          console.warn(
            'ProgressiveMessageList: chat prop is required when not using custom renderMessage'
          );
          return;
        }

        const isUser = isUserMessage(message);

        const messageElement = (
          <div
            key={message.id && `${message.id}-${message.content.length}`}
            className={`relative ${index === 0 && alertsBeforeMessages.length === 0 ? 'mt-0' : 'mt-4'} ${isUser ? 'user' : 'assistant'}`}
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
                  !hasOnlyToolResponses(message) && (
                    <UserMessage message={message} onMessageUpdate={onMessageUpdate} />
                  )
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

        renderedElements.push(messageElement);
      }

      // Add alerts that come after this message
      alertsBetween.forEach((alert, idx) => {
        renderedElements.push(
          <div key={`alert-${alert.timestamp}-${idx}`} className="mt-4">
            <SystemAlertMessage
              message={alert.message}
              level={alert.level as 'info' | 'warning' | 'error' | 'success'}
              timestamp={alert.timestamp}
            />
          </div>
        );
      });
    });

    return renderedElements;
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
    hasContextHandlerContent,
    getContextHandlerType,
    onScrollToBottom,
    systemAlerts,
    hasOnlyToolResponses,
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
