/**
 * VirtualizedMessageList Component
 *
 * Windowed rendering for very large sessions: only the messages near the
 * viewport exist in the DOM, so open time and memory stay flat regardless of
 * session length. BaseChat selects this component above a message-count
 * threshold; ProgressiveMessageList remains the path for normal sessions.
 *
 * Per-message rendering mirrors ProgressiveMessageList's default logic
 * (system notifications, model-change disclosures, tool-call chain styling,
 * user/assistant rendering) so the two paths stay visually equivalent.
 *
 * Known tradeoffs at this size (documented, not bugs):
 * - Find-in-page (Cmd/Ctrl+F) only matches messages currently materialized
 *   in the DOM; off-screen matches are not found.
 * - The list mounts bottom-anchored (chat convention).
 */

import { useLayoutEffect, useMemo, useRef, useState } from 'react';
import { Virtuoso, type VirtuosoHandle } from 'react-virtuoso';
import { useIntl, defineMessages } from '../i18n';
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
import type { Message, NotificationEvent, SystemNotificationContent } from '../types/message';
import { ChatType } from '../types/chat';
import { identifyConsecutiveToolCalls, isInChain } from '../utils/toolCallChaining';
import { getModelDisplayName } from './settings/models/predefinedModelsUtils';

const i18n = defineMessages({
  modelChanged: {
    id: 'progressiveMessageList.modelChanged',
    defaultMessage: 'Model changed: {previousModel} → {currentModel}',
  },
});

interface VirtualizedMessageListProps {
  messages: Message[];
  chat: Pick<ChatType, 'sessionId'>;
  toolCallNotifications?: Map<string, NotificationEvent[]>;
  append?: (value: string) => void;
  isUserMessage: (message: Message) => boolean;
  isStreamingMessage?: boolean;
  onMessageUpdate?: (messageId: string, newContent: string, editType?: 'fork' | 'edit') => void;
  onRenderingComplete?: () => void;
  submitElicitationResponse?: (
    elicitationId: string,
    userData: Record<string, unknown>
  ) => Promise<boolean>;
}

interface RenderItem {
  message: Message;
  origIndex: number;
  notification?: SystemNotificationContent;
  previousModel?: string;
  currentModel?: string;
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

export default function VirtualizedMessageList({
  messages,
  chat,
  toolCallNotifications = new Map(),
  append = () => {},
  isUserMessage,
  isStreamingMessage = false,
  onMessageUpdate,
  onRenderingComplete,
  submitElicitationResponse,
}: VirtualizedMessageListProps) {
  const intl = useIntl();
  const virtuosoRef = useRef<VirtuosoHandle>(null);
  const anchorRef = useRef<HTMLDivElement>(null);
  const [scrollParent, setScrollParent] = useState<HTMLElement | undefined>(undefined);

  // Reuse the surrounding radix ScrollArea viewport as the scroller so the
  // existing ScrollAreaHandle contract (scrollToBottom / isFollowing) keeps
  // working unchanged in BaseChat.
  useLayoutEffect(() => {
    const viewport = anchorRef.current?.closest(
      '[data-radix-scroll-area-viewport]'
    ) as HTMLElement | null;
    if (viewport) {
      setScrollParent(viewport);
    }
  }, []);

  const toolCallChains = useMemo(() => identifyConsecutiveToolCalls(messages), [messages]);

  // One pass over the full array: filter to renderable items and precompute
  // the model-change disclosures (ProgressiveMessageList does a backward scan
  // per message; at this scale it must be a single forward pass).
  const renderItems = useMemo<RenderItem[]>(() => {
    const items: RenderItem[] = [];
    let lastResolvedModel: string | null = null;
    for (let i = 0; i < messages.length; i++) {
      const message = messages[i];
      if (!message.metadata.userVisible) continue;

      const notification = getSystemNotification(message);
      if (notification) {
        items.push({ message, origIndex: i, notification });
        continue;
      }

      const isUser = isUserMessage(message);
      if (isUser && message.content.every((c) => c.type === 'toolResponse')) {
        continue; // renders nothing in the progressive path either
      }

      let previousModel: string | undefined;
      let currentModel: string | undefined;
      if (message.role === 'assistant') {
        const resolved = message.metadata.inference?.resolvedModel ?? null;
        if (resolved) {
          if (lastResolvedModel && lastResolvedModel !== resolved) {
            previousModel = lastResolvedModel;
            currentModel = resolved;
          }
          lastResolvedModel = resolved;
        }
      }
      items.push({ message, origIndex: i, previousModel, currentModel });
    }
    return items;
  }, [messages, isUserMessage]);

  // Rendering is effectively instant under virtualization; honor the
  // completion contract BaseChat uses for its initial scroll-to-bottom.
  const completedRef = useRef(false);
  useLayoutEffect(() => {
    if (!completedRef.current && onRenderingComplete) {
      completedRef.current = true;
      setTimeout(() => onRenderingComplete(), 50);
    }
  }, [onRenderingComplete]);

  const lastMessage = messages[messages.length - 1];

  return (
    <div ref={anchorRef} data-testid="virtualized-message-list">
      {scrollParent && (
        <Virtuoso
          ref={virtuosoRef}
          data={renderItems}
          customScrollParent={scrollParent}
          initialTopMostItemIndex={{ index: Math.max(0, renderItems.length - 1), align: 'end' }}
          increaseViewportBy={{ top: 1200, bottom: 1200 }}
          computeItemKey={(_index, item) =>
            item.message.id ?? `msg-${item.origIndex}-${item.message.created}`
          }
          itemContent={(_index: number, item: RenderItem) => {
            const { message, origIndex, notification } = item;

            if (notification) {
              return (
                <div
                  className={`relative ${origIndex === 0 ? 'mt-0' : 'mt-4'} assistant`}
                  data-testid="message-container"
                >
                  {renderSystemNotification(notification)}
                </div>
              );
            }

            const isUser = isUserMessage(message);
            const messageIsInChain = isInChain(origIndex, toolCallChains);

            return (
              <>
                {item.previousModel && item.currentModel && (
                  <SystemNotificationInline
                    notification={{
                      msg: intl.formatMessage(i18n.modelChanged, {
                        previousModel: getModelDisplayName(item.previousModel),
                        currentModel: getModelDisplayName(item.currentModel),
                      }),
                      notificationType: 'inlineMessage',
                    }}
                  />
                )}
                <div
                  className={`relative ${origIndex === 0 ? 'mt-0' : 'mt-4'} ${isUser ? 'user' : 'assistant'} ${messageIsInChain ? 'in-chain' : ''}`}
                  data-testid="message-container"
                >
                  {isUser ? (
                    <UserMessage message={message} onMessageUpdate={onMessageUpdate} />
                  ) : (
                    <GooseMessage
                      sessionId={chat.sessionId}
                      message={message}
                      messages={messages}
                      append={append}
                      toolCallNotifications={toolCallNotifications}
                      isStreaming={
                        isStreamingMessage &&
                        message === lastMessage &&
                        message.role === 'assistant'
                      }
                      submitElicitationResponse={submitElicitationResponse}
                    />
                  )}
                </div>
              </>
            );
          }}
        />
      )}
    </div>
  );
}
