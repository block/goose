/**
 * ProgressiveMessageList Component
 *
 * A performance-optimized message list that renders messages from the end
 * (newest messages first) and loads older messages on scroll to top.
 * This prevents UI blocking when loading long chat sessions and ensures
 * the user sees the most recent messages immediately.
 *
 * Key Features:
 * - Renders newest messages first (last N messages)
 * - Loads older messages when scrolling to top
 * - Preserves scroll position when prepending older messages
 * - "Load all" on Cmd/Ctrl+F for search
 * - Smooth user experience with responsive UI
 * - Configurable batch size
 */

import { Fragment, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { defineMessages, useIntl } from '../i18n';
import GooseMessage from './GooseMessage';
import UserMessage from './UserMessage';
import {
  SystemNotificationInline,
  getInlineSystemNotification,
} from './context_management/SystemNotificationInline';
import {
  CreditsExhaustedNotification,
  getCreditsExhaustedNotification,
} from './context_management/CreditsExhaustedNotification';
import type {
  ImageData,
  Message,
  NotificationEvent,
  SystemNotificationContent,
} from '../types/message';
import LoadingGoose from './LoadingGoose';
import { ChatType } from '../types/chat';
import { identifyConsecutiveToolCalls, isInChain } from '../utils/toolCallChaining';
import { getModelDisplayName } from './settings/models/predefinedModelsUtils';

const i18n = defineMessages({
  loadingMessages: {
    id: 'progressiveMessageList.loadingMessages',
    defaultMessage: 'Loading messages...',
  },
  loadOlder: {
    id: 'progressiveMessageList.loadOlder',
    defaultMessage: 'Load {count} older messages',
  },
  loadAll: {
    id: 'progressiveMessageList.loadAll',
    defaultMessage: 'Load all {count} messages',
  },
  searchHint: {
    id: 'progressiveMessageList.searchHint',
    defaultMessage: 'Press Cmd/Ctrl+F to load all messages immediately for search',
  },
  modelChanged: {
    id: 'progressiveMessageList.modelChanged',
    defaultMessage: 'Model changed: {previousModel} \u2192 {currentModel}',
  },
});

interface ProgressiveMessageListProps {
  messages: Message[];
  chat: Pick<ChatType, 'sessionId'>;
  toolCallNotifications?: Map<string, NotificationEvent[]>; // Make optional
  append?: (value: string) => void; // Make optional
  isUserMessage: (message: Message) => boolean;
  batchSize?: number;
  showLoadingThreshold?: number; // Only use windowed rendering if more than X messages
  // Custom render function for messages
  renderMessage?: (message: Message, index: number) => React.ReactNode | null;
  isStreamingMessage?: boolean; // Whether messages are currently being streamed
  onMessageUpdate?: (
    messageId: string,
    newContent: string,
    editType: 'fork' | 'edit',
    retainedImages: ImageData[]
  ) => void;
  onRenderingComplete?: () => void; // Callback when initial rendering is done
  submitElicitationResponse?: (
    elicitationId: string,
    userData: Record<string, unknown>
  ) => Promise<boolean>;
}

export default function ProgressiveMessageList({
  messages,
  chat,
  toolCallNotifications = new Map(),
  append = () => {},
  isUserMessage,
  batchSize = 30,
  showLoadingThreshold = 50,
  renderMessage,
  isStreamingMessage = false,
  onMessageUpdate,
  onRenderingComplete,
  submitElicitationResponse,
}: ProgressiveMessageListProps) {
  const intl = useIntl();

  // For short lists, show all messages from the start.
  // For long lists, start from the end (show newest messages).
  const isShortList = messages.length <= showLoadingThreshold;

  // Index of the first message to render (0 = show all from beginning).
  // For long lists we start from the end.
  const [visibleStartIndex, setVisibleStartIndex] = useState(() =>
    isShortList ? 0 : Math.max(0, messages.length - showLoadingThreshold)
  );

  const loadingOlderRef = useRef(false);
  const [showLoadingIndicator, setShowLoadingIndicator] = useState(false);
  const sentinelRef = useRef<HTMLDivElement>(null);
  const viewportRef = useRef<HTMLDivElement | null>(null);
  const pendingScrollRestoreRef = useRef<{
    oldScrollHeight: number;
    oldScrollTop: number;
  } | null>(null);
  // Track previous message length to detect additions/removals
  const prevMessagesLengthRef = useRef(messages.length);
  // Global search listener ref (to avoid stale closure over visibleStartIndex)
  const visibleStartIndexRef = useRef(visibleStartIndex);
  visibleStartIndexRef.current = visibleStartIndex;

  // Find the scrollable viewport (Radix scroll-area viewport) on mount
  useEffect(() => {
    const sentinel = sentinelRef.current;
    if (!sentinel) return;

    // Walk up to find the scrollable parent
    let el: HTMLElement | null = sentinel.parentElement;
    while (el) {
      if (el.classList.contains('radix-scroll-area-viewport')) {
        viewportRef.current = el;
        return;
      }
      // Fallback: check if this element is scrollable
      if (el.scrollHeight > el.clientHeight) {
        const style = getComputedStyle(el);
        if (style.overflow === 'auto' || style.overflowY === 'scroll' || style.overflow === 'scroll') {
          viewportRef.current = el;
          return;
        }
      }
      el = el.parentElement;
    }
  }, []);

  // Call onRenderingComplete when messages are first loaded, so BaseChat
  // can auto-scroll to the bottom to show the most recent messages.
  useEffect(() => {
    if (!onRenderingComplete) return;
    if (messages.length === 0) return;
    const timer = setTimeout(() => onRenderingComplete(), 50);
    return () => clearTimeout(timer);
    // Re-fire only when messages go from empty to non-empty
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [messages.length > 0]);

  // IntersectionObserver: detect when user scrolls near the top → load older messages
  useEffect(() => {
    if (isShortList || visibleStartIndex === 0 || loadingOlderRef.current) return;

    const sentinel = sentinelRef.current;
    if (!sentinel) return;

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting) {
          // Load more older messages (guard against rapid re-triggering)
          if (loadingOlderRef.current) return;
          loadingOlderRef.current = true;
          setVisibleStartIndex((prev: number) => {
            if (prev <= 0) {
              loadingOlderRef.current = false;
              return 0;
            }
            const viewport = viewportRef.current;
            pendingScrollRestoreRef.current = {
              oldScrollHeight: viewport?.scrollHeight ?? 0,
              oldScrollTop: viewport?.scrollTop ?? 0,
            };
            return Math.max(0, prev - batchSize);
          });
        }
      },
      { rootMargin: '300px 0px 0px 0px' }
    );

    observer.observe(sentinel);
    return () => observer.disconnect();
  }, [isShortList, visibleStartIndex, batchSize]);

  // After rendering with updated visibleStartIndex, restore scroll position
  // so prepended content doesn't jolt the viewport.
  useEffect(() => {
    const pending = pendingScrollRestoreRef.current;
    if (!pending) return;
    pendingScrollRestoreRef.current = null;

    // Reset loading guard after render completes
    loadingOlderRef.current = false;

    // Find the viewport again (it may have been remounted)
    const viewport = viewportRef.current;
    if (viewport && pending.oldScrollHeight > 0) {
      const newScrollHeight = viewport.scrollHeight;
      const heightDiff = newScrollHeight - pending.oldScrollHeight;
      if (heightDiff > 0) {
        viewport.scrollTop = pending.oldScrollTop + heightDiff;
      }
    }
  }, [visibleStartIndex]);

  // When messages length changes (streaming, truncation, or initial load), adjust window
  useEffect(() => {
    if (isShortList) {
      setVisibleStartIndex(0);
      return;
    }

    // If messages just arrived (jumped from empty to many) and we're showing from index 0
    // but there are more messages than the threshold, switch to showing only the last ones.
    // During streaming we don't adjust — keep showing all messages.
    if (visibleStartIndex === 0 && messages.length > showLoadingThreshold && prevMessagesLengthRef.current === 0) {
      setVisibleStartIndex(messages.length - showLoadingThreshold);
      return;
    }

    // If all messages are already visible, keep showing all
    if (visibleStartIndex === 0) return;

    // If messages were removed (truncation), clamp visibleStartIndex
    if (messages.length < prevMessagesLengthRef.current) {
      setVisibleStartIndex((prev: number) =>
        Math.min(prev, Math.max(0, messages.length - batchSize))
      );
    }

    prevMessagesLengthRef.current = messages.length;
  }, [messages.length, isShortList, visibleStartIndex, batchSize, showLoadingThreshold]);

  // Force complete rendering (show all messages) when search shortcut is pressed
  useEffect(() => {
    if (isShortList || visibleStartIndex === 0) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      const isMac = window.electron.platform === 'darwin';
      const isSearchShortcut = (isMac ? e.metaKey : e.ctrlKey) && e.key === 'f';

      if (isSearchShortcut) {
        setVisibleStartIndex(0);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isShortList, visibleStartIndex]);

  // Handle "load all" button click
  const handleLoadAll = useCallback(() => {
    setVisibleStartIndex(0);
  }, []);

  const doLoadOlder = useCallback(() => {
    setVisibleStartIndex((prev: number) => {
      if (prev <= 0) return 0;
      const viewport = viewportRef.current;
      pendingScrollRestoreRef.current = {
        oldScrollHeight: viewport?.scrollHeight ?? 0,
        oldScrollTop: viewport?.scrollTop ?? 0,
      };
      return Math.max(0, prev - batchSize);
    });
  }, [batchSize]);

  // Handle "load older" button click
  const handleLoadOlder = useCallback(() => {
    if (loadingOlderRef.current) return;
    loadingOlderRef.current = true;
    doLoadOlder();
  }, [doLoadOlder]);

  // Show/hide loading indicator when loading older messages
  useEffect(() => {
    setShowLoadingIndicator(loadingOlderRef.current);
  }, [visibleStartIndex]);

  const hasOnlyToolResponses = (message: Message) =>
    message.content.every((c) => c.type === 'toolResponse');

  const getResolvedModel = useCallback((message: Message): string | null => {
    if (message.role !== 'assistant' || !message.metadata.userVisible) return null;
    return message.metadata.inference?.resolvedModel ?? null;
  }, []);

  const getPreviousResolvedModel = useCallback(
    (index: number): string | null => {
      // index is the full message array index
      for (let i = index - 1; i >= 0; i--) {
        const model = getResolvedModel(messages[i]);
        if (model) return model;
      }
      return null;
    },
    [getResolvedModel, messages]
  );

  const renderModelChangeDisclosure = useCallback(
    (previousModel: string, currentModel: string) => (
      <SystemNotificationInline
        notification={{
          msg: intl.formatMessage(i18n.modelChanged, {
            previousModel: getModelDisplayName(previousModel),
            currentModel: getModelDisplayName(currentModel),
          }),
          notificationType: 'inlineMessage',
        }}
      />
    ),
    [intl]
  );

  const getSystemNotification = (message: Message): SystemNotificationContent | undefined => {
    return getCreditsExhaustedNotification(message) ?? getInlineSystemNotification(message);
  };

  const renderSystemNotification = (notification: SystemNotificationContent) => {
    switch (notification.notificationType) {
      case 'creditsExhausted':
        return <CreditsExhaustedNotification notification={notification} />;
      case 'inlineMessage':
        return <SystemNotificationInline notification={notification} />;
      default:
        return null;
    }
  };

  // Detect tool call chains using the full messages array
  const toolCallChains = useMemo(() => identifyConsecutiveToolCalls(messages), [messages]);

  // Messages to render (slice from visibleStartIndex to end)
  const messagesToRender = isShortList ? messages : messages.slice(visibleStartIndex);
  const isShowingAll = isShortList || visibleStartIndex === 0;
  const hiddenCount = messages.length - messagesToRender.length;

  // Render messages
  const renderMessages = useCallback(() => {
    return messagesToRender
      .map((message, visibleIndex) => {
        // Calculate the index in the FULL messages array
        const fullIndex = isShortList ? visibleIndex : visibleStartIndex + visibleIndex;

        if (!message.metadata.userVisible) {
          return null;
        }
        if (renderMessage) {
          return renderMessage(message, fullIndex);
        }

        // Default rendering logic (for BaseChat)
        if (!chat) {
          console.warn(
            'ProgressiveMessageList: chat prop is required when not using custom renderMessage'
          );
          return null;
        }

        const notification = getSystemNotification(message);
        if (notification) {
          return (
            <div
              key={`notification-${message.id ?? `msg-${fullIndex}-${message.created}`}`}
              className={`relative ${fullIndex === 0 ? 'mt-0' : 'mt-4'} assistant`}
              data-testid="message-container"
            >
              {renderSystemNotification(notification)}
            </div>
          );
        }

        const isUser = isUserMessage(message);
        const messageIsInChain = isInChain(fullIndex, toolCallChains);
        const currentResolvedModel = getResolvedModel(message);
        const previousResolvedModel = currentResolvedModel
          ? getPreviousResolvedModel(fullIndex)
          : null;
        const showModelChangeDisclosure = Boolean(
          currentResolvedModel &&
          previousResolvedModel &&
          currentResolvedModel !== previousResolvedModel
        );

        const messageKey = message.id ?? `msg-${fullIndex}-${message.created}`;

        return (
          <Fragment key={messageKey}>
            {showModelChangeDisclosure &&
              currentResolvedModel &&
              previousResolvedModel &&
              renderModelChangeDisclosure(previousResolvedModel, currentResolvedModel)}
            <div
              className={`relative ${fullIndex === 0 ? 'mt-0' : 'mt-4'} ${isUser ? 'user' : 'assistant'} ${messageIsInChain ? 'in-chain' : ''}`}
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
                    fullIndex === messages.length - 1 &&
                    message.role === 'assistant'
                  }
                  submitElicitationResponse={submitElicitationResponse}
                />
              )}
            </div>
          </Fragment>
        );
      })
      .filter(Boolean);
  }, [
    messagesToRender,
    messages,
    visibleStartIndex,
    isShortList,
    renderMessage,
    isUserMessage,
    chat,
    append,
    toolCallNotifications,
    isStreamingMessage,
    onMessageUpdate,
    toolCallChains,
    submitElicitationResponse,
    getPreviousResolvedModel,
    getResolvedModel,
    renderModelChangeDisclosure,
  ]);

  return (
    <>
      {/* Sentinel for scroll-to-top detection (hidden when all messages are visible) */}
      {!isShortList && !isShowingAll && (
        <div
          ref={sentinelRef}
          className="h-4 -mt-4"
          aria-hidden="true"
          data-testid="load-older-sentinel"
        />
      )}

      {/* "Load older" section — shown when some messages are hidden */}
      {!isShowingAll && hiddenCount > 0 && (
        <div className="flex flex-col items-center py-4 px-4">
          <button
            type="button"
            onClick={() => {
              handleLoadOlder();
            }}
            className="text-sm text-accent hover:text-accent-hover underline transition-colors cursor-pointer"
          >
            {intl.formatMessage(i18n.loadOlder, { count: Math.min(hiddenCount, batchSize) })}
          </button>
          {showLoadingIndicator && (
            <div className="mt-2">
              <LoadingGoose
                message={intl.formatMessage(i18n.loadingMessages)}
              />
            </div>
          )}
          <button
            type="button"
            onClick={handleLoadAll}
            className="text-xs text-text-secondary hover:text-text-primary underline transition-colors mt-1 cursor-pointer"
          >
            {intl.formatMessage(i18n.loadAll, { count: hiddenCount })}
          </button>
          <div className="text-xs text-text-secondary mt-1">
            {intl.formatMessage(i18n.searchHint)}
          </div>
        </div>
      )}

      {renderMessages()}
    </>
  );
}