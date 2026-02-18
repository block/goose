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

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { Message } from '../api';
import GooseMessage from './GooseMessage';
import UserMessage from './UserMessage';
import { SystemNotificationInline } from './context_management/SystemNotificationInline';
import type { NotificationEvent } from '../types/message';
import LoadingGoose from './LoadingGoose';
import type { ChatType } from '../types/chat';
import { identifyConsecutiveToolCalls, isInChain } from '../utils/toolCallChaining';
import { identifyWorkBlocks } from '../utils/assistantWorkBlocks';
import WorkBlockIndicator from './WorkBlockIndicator';

interface ProgressiveMessageListProps {
  messages: Message[];
  chat: Pick<ChatType, 'sessionId'>;
  toolCallNotifications?: Map<string, NotificationEvent[]>; // Make optional
  append?: (value: string) => void; // Make optional
  isUserMessage: (message: Message) => boolean;
  batchSize?: number;
  batchDelay?: number;
  showLoadingThreshold?: number; // Only show loading if more than X messages
  // Custom render function for messages
  renderMessage?: (message: Message, index: number) => React.ReactNode | null;
  isStreamingMessage?: boolean; // Whether messages are currently being streamed
  onMessageUpdate?: (messageId: string, newContent: string) => void;
  onRenderingComplete?: () => void; // Callback when all messages are rendered
  submitElicitationResponse?: (
    elicitationId: string,
    userData: Record<string, unknown>
  ) => Promise<void>;
}

export default function ProgressiveMessageList({
  messages,
  chat,
  toolCallNotifications = new Map(),
  append = () => {},
  isUserMessage,
  batchSize = 20,
  batchDelay = 20,
  showLoadingThreshold = 50,
  renderMessage, // Custom render function
  isStreamingMessage = false, // Whether messages are currently being streamed
  onMessageUpdate,
  onRenderingComplete,
  submitElicitationResponse,
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

  const hasInlineSystemNotification = (message: Message): boolean => {
    return message.content.some(
      (content) =>
        content.type === 'systemNotification' && content.notificationType === 'inlineMessage'
    );
  };

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

  // Detect tool call chains
  const toolCallChains = useMemo(() => identifyConsecutiveToolCalls(messages), [messages]);

  // Compute work blocks for collapsing intermediate assistant messages
  const workBlocks = useMemo(
    () => identifyWorkBlocks(messages, isStreamingMessage),
    [messages, isStreamingMessage]
  );

  // Render messages up to the current rendered count
  const renderMessages = useCallback(() => {
    const messagesToRender = messages.slice(0, renderedCount);
    const seenBlocks = new Set<string>();

    return messagesToRender
      .map((message, index) => {
        if (!message.metadata.userVisible) {
          return null;
        }
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

        // System notifications are never user messages, handle them first
        if (hasInlineSystemNotification(message)) {
          return (
            <div
              key={message.id ?? `msg-${index}-${message.created}`}
              className={`relative ${index === 0 ? 'mt-0' : 'mt-4'} assistant`}
              data-testid="message-container"
            >
              <SystemNotificationInline message={message} />
            </div>
          );
        }

        // Check if this message is part of a work block
        const block = workBlocks.get(index);
        if (block) {
          // This message is an intermediate message in a work block — collapse it
          const blockKey = `block-${block.intermediateIndices[0]}`;

          if (!seenBlocks.has(blockKey)) {
            // First message of this block — render the WorkBlockIndicator
            seenBlocks.add(blockKey);
            const blockMessages = block.intermediateIndices.map((i: number) => messages[i]);

            return (
              <div
                key={blockKey}
                className="relative mt-2 assistant"
                data-testid="work-block-indicator"
              >
                <WorkBlockIndicator
                  messages={blockMessages}
                  blockId={blockKey}
                  isStreaming={block.isStreaming}
                  sessionId={chat.sessionId}
                  toolCallNotifications={toolCallNotifications}
                />
              </div>
            );
          }
          // Subsequent messages in the block — hide them
          return null;
        }

        const isUser = isUserMessage(message);
        const messageIsInChain = isInChain(index, toolCallChains);

        return (
          <div
            key={message.id ?? `msg-${index}-${message.created}`}
            className={`relative ${index === 0 ? 'mt-0' : 'mt-4'} ${isUser ? 'user' : 'assistant'} ${messageIsInChain ? 'in-chain' : ''}`}
            data-testid="message-container"
          >
            {isUser ? (
              !hasOnlyToolResponses(message) && (
                <UserMessage message={message} onMessageUpdate={onMessageUpdate} />
              )
            ) : (
              <GooseMessage
                sessionId={chat.sessionId}
                message={message}
                messages={messages}
                append={append}
                toolCallNotifications={toolCallNotifications}
                isStreaming={
                  isStreamingMessage &&
                  !isUser &&
                  index === messagesToRender.length - 1 &&
                  message.role === 'assistant'
                }
                suppressToolCalls={(() => {
                  // 1. Suppress on work block final answer (tool calls already in the indicator)
                  for (const block of workBlocks.values()) {
                    if (block.finalIndex === index) return true;
                  }
                  // 2. During streaming, suppress tool calls on assistant messages
                  //    in the active streaming run — they'll be collapsed into a
                  //    WorkBlockIndicator once the block is recognized. This prevents
                  //    the "transient flash" of raw tool calls before collapse.
                  if (isStreamingMessage && message.role === 'assistant') {
                    const hasTools = message.content.some(
                      (c: { type: string }) => c.type === 'toolRequest'
                    );
                    if (hasTools && !workBlocks.has(index)) {
                      return true;
                    }
                  }
                  return false;
                })()}
                submitElicitationResponse={submitElicitationResponse}
              />
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
    toolCallNotifications,
    isStreamingMessage,
    onMessageUpdate,
    toolCallChains,
    workBlocks,
    submitElicitationResponse,
  ]);

  // Show pending indicator when streaming started but no assistant response yet
  // Don't show if there's already an active streaming work block
  const lastMessage = messages[messages.length - 1];
  const hasNoAssistantResponse = !lastMessage || lastMessage.role === 'user';
  const hasStreamingWorkBlock = Array.from(workBlocks.values()).some(
    (b) => b.isStreaming
  );
  const showPendingIndicator =
    isStreamingMessage &&
    messages.length > 0 &&
    hasNoAssistantResponse &&
    !hasStreamingWorkBlock;

  return (
    <>
      {renderMessages()}

      {/* Pending streaming indicator — shows immediately after user sends */}
      {showPendingIndicator && (
        <div className="relative mt-4 assistant">
          <WorkBlockIndicator
            messages={[]}
            blockId="pending"
            isStreaming={true}
            sessionId={chat.sessionId}
          />
        </div>
      )}

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
