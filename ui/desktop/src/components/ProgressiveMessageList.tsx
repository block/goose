/**
 * ProgressiveMessageList Component
 *
 * Renders a window of the transcript instead of mounting every historical
 * message. Long sessions start at the latest N messages; older history is
 * paged in with "Show earlier" or expanded fully for search.
 */

import { Fragment, memo, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
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
import { ChatType } from '../types/chat';
import {
  buildToolCallLookups,
  getPreviousResolvedModels,
  identifyConsecutiveToolCalls,
  isInChain,
  messageNotificationsChanged,
  messageToolLookupsChanged,
  shouldHideTimestamp,
  type ToolCallLookups,
} from '../utils/toolCallChaining';
import { getModelDisplayName } from './settings/models/predefinedModelsUtils';
import {
  DEFAULT_VISIBLE_MESSAGE_WINDOW,
  earlierTranscriptWindowStart,
  initialTranscriptWindowStart,
  transcriptMessageKey,
  visibleTranscriptWindowStart,
} from '../utils/transcriptWindow';

const i18n = defineMessages({
  showEarlier: {
    id: 'progressiveMessageList.showEarlier',
    defaultMessage: 'Show earlier messages ({hiddenCount} hidden)',
  },
  searchHint: {
    id: 'progressiveMessageList.searchHint',
    defaultMessage: 'Press Cmd/Ctrl+F to load all messages immediately for search',
  },
  modelChanged: {
    id: 'progressiveMessageList.modelChanged',
    defaultMessage: 'Model changed: {previousModel} → {currentModel}',
  },
});

interface ProgressiveMessageListProps {
  messages: Message[];
  chat: Pick<ChatType, 'sessionId'>;
  toolCallNotifications?: Map<string, NotificationEvent[]>;
  append?: (value: string) => void;
  isUserMessage: (message: Message) => boolean;
  visibleWindow?: number;
  renderMessage?: (message: Message, index: number) => React.ReactNode | null;
  isStreamingMessage?: boolean;
  onMessageUpdate?: (
    messageId: string,
    newContent: string,
    editType: 'fork' | 'edit',
    retainedImages: ImageData[]
  ) => void;
  onRenderingComplete?: () => void;
  submitElicitationResponse?: (
    elicitationId: string,
    userData: Record<string, unknown>
  ) => Promise<boolean>;
  hasEarlierMessages?: boolean;
  onLoadEarlierMessages?: () => Promise<void> | void;
}

const EMPTY_TOOL_CALL_NOTIFICATIONS = new Map<string, NotificationEvent[]>();

function noopAppend() {}

function hasOnlyToolResponses(message: Message) {
  return message.content.every((content) => content.type === 'toolResponse');
}

function getSystemNotification(message: Message): SystemNotificationContent | undefined {
  return getCreditsExhaustedNotification(message) ?? getInlineSystemNotification(message);
}

function renderSystemNotification(notification: SystemNotificationContent) {
  switch (notification.notificationType) {
    case 'creditsExhausted':
      return <CreditsExhaustedNotification notification={notification} />;
    case 'inlineMessage':
      return <SystemNotificationInline notification={notification} />;
    default:
      return null;
  }
}

interface MessageRowProps {
  message: Message;
  index: number;
  transcriptAnchor?: string;
  chatSessionId: string;
  isUser: boolean;
  messageIsInChain: boolean;
  hideTimestamp: boolean;
  currentResolvedModel: string | null;
  previousResolvedModel: string | null;
  lookups: ToolCallLookups;
  toolCallNotifications: Map<string, NotificationEvent[]>;
  append: (value: string) => void;
  isStreaming: boolean;
  onMessageUpdate?: ProgressiveMessageListProps['onMessageUpdate'];
  submitElicitationResponse?: ProgressiveMessageListProps['submitElicitationResponse'];
}

function messageRowPropsAreEqual(previous: MessageRowProps, next: MessageRowProps): boolean {
  if (
    previous.message !== next.message ||
    previous.index !== next.index ||
    previous.transcriptAnchor !== next.transcriptAnchor ||
    previous.chatSessionId !== next.chatSessionId ||
    previous.isUser !== next.isUser ||
    previous.messageIsInChain !== next.messageIsInChain ||
    previous.hideTimestamp !== next.hideTimestamp ||
    previous.currentResolvedModel !== next.currentResolvedModel ||
    previous.previousResolvedModel !== next.previousResolvedModel ||
    previous.append !== next.append ||
    previous.isStreaming !== next.isStreaming ||
    previous.onMessageUpdate !== next.onMessageUpdate ||
    previous.submitElicitationResponse !== next.submitElicitationResponse
  ) {
    return false;
  }

  return (
    !messageToolLookupsChanged(next.message, previous.lookups, next.lookups) &&
    !messageNotificationsChanged(
      next.message,
      previous.toolCallNotifications,
      next.toolCallNotifications
    )
  );
}

const MessageRow = memo(function MessageRow({
  message,
  index,
  transcriptAnchor,
  chatSessionId,
  isUser,
  messageIsInChain,
  hideTimestamp,
  currentResolvedModel,
  previousResolvedModel,
  lookups,
  toolCallNotifications,
  append,
  isStreaming,
  onMessageUpdate,
  submitElicitationResponse,
}: MessageRowProps) {
  const intl = useIntl();
  const notification = getSystemNotification(message);
  if (notification) {
    return (
      <div
        className={`relative ${index === 0 ? 'mt-0' : 'mt-4'} assistant`}
        data-testid="message-container"
        data-transcript-anchor={transcriptAnchor}
      >
        {renderSystemNotification(notification)}
      </div>
    );
  }

  const showModelChangeDisclosure = Boolean(
    currentResolvedModel && previousResolvedModel && currentResolvedModel !== previousResolvedModel
  );

  return (
    <Fragment>
      {showModelChangeDisclosure && currentResolvedModel && previousResolvedModel && (
        <SystemNotificationInline
          notification={{
            msg: intl.formatMessage(i18n.modelChanged, {
              previousModel: getModelDisplayName(previousResolvedModel),
              currentModel: getModelDisplayName(currentResolvedModel),
            }),
            notificationType: 'inlineMessage',
          }}
        />
      )}
      <div
        className={`relative ${index === 0 ? 'mt-0' : 'mt-4'} ${isUser ? 'user' : 'assistant'} ${messageIsInChain ? 'in-chain' : ''}`}
        data-testid="message-container"
        data-transcript-anchor={transcriptAnchor}
      >
        {isUser ? (
          !hasOnlyToolResponses(message) && (
            <UserMessage message={message} onMessageUpdate={onMessageUpdate} />
          )
        ) : (
          <GooseMessage
            sessionId={chatSessionId}
            message={message}
            lookups={lookups}
            append={append}
            toolCallNotifications={toolCallNotifications}
            hideTimestamp={hideTimestamp}
            isStreaming={isStreaming}
            submitElicitationResponse={submitElicitationResponse}
          />
        )}
      </div>
    </Fragment>
  );
}, messageRowPropsAreEqual);

function ProgressiveMessageList({
  messages,
  chat,
  toolCallNotifications = EMPTY_TOOL_CALL_NOTIFICATIONS,
  append = noopAppend,
  isUserMessage,
  visibleWindow = DEFAULT_VISIBLE_MESSAGE_WINDOW,
  renderMessage,
  isStreamingMessage = false,
  onMessageUpdate,
  onRenderingComplete,
  submitElicitationResponse,
  hasEarlierMessages = false,
  onLoadEarlierMessages,
}: ProgressiveMessageListProps) {
  const intl = useIntl();
  const [showAllMessages, setShowAllMessages] = useState(false);
  const [pinnedToLiveEdge, setPinnedToLiveEdge] = useState(true);
  const [windowStart, setWindowStart] = useState(() =>
    initialTranscriptWindowStart(messages.length, visibleWindow)
  );
  const pendingScrollRestoreKeyRef = useRef<string | null>(null);

  const effectiveWindowStart = visibleTranscriptWindowStart({
    messageCount: messages.length,
    showAll: showAllMessages,
    pinnedToLiveEdge,
    windowStart,
    visibleWindow,
  });

  useEffect(() => {
    if (!onRenderingComplete) {
      return;
    }
    const timeoutId = window.setTimeout(() => onRenderingComplete(), 50);
    return () => window.clearTimeout(timeoutId);
  }, [messages.length, onRenderingComplete, windowStart]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const isMac = window.electron.platform === 'darwin';
      const isSearchShortcut = (isMac ? event.metaKey : event.ctrlKey) && event.key === 'f';
      if (isSearchShortcut) {
        setShowAllMessages(true);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  const toolCallChains = useMemo(() => identifyConsecutiveToolCalls(messages), [messages]);
  const previousResolvedModels = useMemo(() => getPreviousResolvedModels(messages), [messages]);
  const toolCallLookups = useMemo(() => buildToolCallLookups(messages), [messages]);
  const hiddenCount = effectiveWindowStart;
  const messagesToRender = messages.slice(effectiveWindowStart);
  useLayoutEffect(() => {
    const restoreKey = pendingScrollRestoreKeyRef.current;
    if (!restoreKey) {
      return;
    }
    pendingScrollRestoreKeyRef.current = null;
    const anchor = document.querySelector(
      `[data-transcript-anchor="${restoreKey.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"]`
    );
    if (anchor instanceof HTMLElement && typeof anchor.scrollIntoView === 'function') {
      anchor.scrollIntoView({ block: 'start' });
    }
  }, [hiddenCount, messages.length]);

  return (
    <>
      {(hiddenCount > 0 || hasEarlierMessages) && (
        <div className="flex flex-col items-center justify-center py-3">
          <button
            type="button"
            className="text-xs text-text-secondary hover:text-text-primary underline-offset-2 hover:underline"
            onClick={() => {
              const firstVisible = messages[effectiveWindowStart];
              pendingScrollRestoreKeyRef.current = firstVisible
                ? transcriptMessageKey(firstVisible)
                : null;
              if (hiddenCount === 0 && hasEarlierMessages) {
                setPinnedToLiveEdge(false);
                setWindowStart(0);
                void onLoadEarlierMessages?.();
                return;
              }
              setPinnedToLiveEdge(false);
              setWindowStart(earlierTranscriptWindowStart(effectiveWindowStart, visibleWindow));
            }}
          >
            {intl.formatMessage(i18n.showEarlier, {
              hiddenCount: hiddenCount > 0 ? hiddenCount : 'more',
            })}
          </button>
          <div className="text-xs text-text-muted mt-1">{intl.formatMessage(i18n.searchHint)}</div>
        </div>
      )}

      {messagesToRender.map((message, windowIndex) => {
        const index = effectiveWindowStart + windowIndex;
        if (!message.metadata.userVisible) {
          return null;
        }
        if (renderMessage) {
          return renderMessage(message, index);
        }

        const currentResolvedModel =
          message.role === 'assistant' && message.metadata.userVisible
            ? (message.metadata.inference?.resolvedModel ?? null)
            : null;
        const messageKey = transcriptMessageKey(message);

        return (
          <MessageRow
            key={messageKey}
            message={message}
            index={index}
            transcriptAnchor={messageKey}
            chatSessionId={chat.sessionId}
            isUser={isUserMessage(message)}
            messageIsInChain={isInChain(index, toolCallChains)}
            hideTimestamp={shouldHideTimestamp(index, toolCallChains)}
            currentResolvedModel={currentResolvedModel}
            previousResolvedModel={
              currentResolvedModel ? (previousResolvedModels[index] ?? null) : null
            }
            lookups={toolCallLookups}
            toolCallNotifications={toolCallNotifications}
            append={append}
            isStreaming={
              isStreamingMessage &&
              index === messages.length - 1 &&
              message.role === 'assistant'
            }
            onMessageUpdate={onMessageUpdate}
            submitElicitationResponse={submitElicitationResponse}
          />
        );
      })}
    </>
  );
}

export default memo(ProgressiveMessageList);
