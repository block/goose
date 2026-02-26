/**
 * SSE stream decoder: parses server-sent events and dispatches state updates.
 *
 * Extracted from useChatStream to isolate event parsing and batching logic.
 * Handles reduced-motion preferences with batched UI updates.
 */
import type React from 'react';
import type { Message, MessageEvent, TokenState } from '@/api';
import { ChatState } from '@/types/chatState';
import type { MessageWithAttribution, NotificationEvent, RoutingInfo } from '@/types/message';
import { getCompactingMessage, getThinkingMessage } from '@/types/message';
import { errorMessage } from '@/utils/conversionUtils';
import { maybeHandlePlatformEvent } from '@/utils/platform_events';
import type { StreamAction } from './streamReducer';

// ── Helpers ──────────────────────────────────────────────────────────

interface ModelInfo {
  model: string;
  mode: string;
}

export function pushMessage(currentMessages: Message[], incomingMsg: Message): Message[] {
  const lastMsg = currentMessages[currentMessages.length - 1];
  if (lastMsg && lastMsg.role === incomingMsg.role) {
    const lastContent = lastMsg.content[lastMsg.content.length - 1];
    const newContent = incomingMsg.content[incomingMsg.content.length - 1];

    if (
      lastContent?.type === 'text' &&
      newContent?.type === 'text' &&
      lastMsg.id === incomingMsg.id
    ) {
      const accumulatedContent = [...lastMsg.content];
      const lastIdx = accumulatedContent.length - 1;
      accumulatedContent[lastIdx] = {
        ...lastContent,
        text: (lastContent as { text: string }).text + (newContent as { text: string }).text,
      };
      currentMessages[currentMessages.length - 1] = {
        ...incomingMsg,
        content: accumulatedContent,
      };
      return currentMessages;
    }
  }
  currentMessages.push(incomingMsg);
  return currentMessages;
}

export function prefersReducedMotion(): boolean {
  return window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;
}

const REDUCED_MOTION_BATCH_INTERVAL = 1000;

// ── Stream decoder ───────────────────────────────────────────────────

export async function streamFromResponse(
  stream: AsyncIterable<MessageEvent>,
  initialMessages: Message[],
  dispatch: React.Dispatch<StreamAction>,
  onFinish: (error?: string) => void,
  sessionId: string
): Promise<void> {
  const reduceMotion = prefersReducedMotion();

  let currentMessages = [...initialMessages];
  let currentModelInfo: ModelInfo | null = null;
  let currentRoutingInfo: RoutingInfo | null = null;
  let latestTokenState: TokenState | null = null;
  let latestChatState: ChatState = ChatState.Streaming;
  let hasPendingUpdate = false;
  let lastBatchUpdate = Date.now();

  const flushBatchedUpdates = () => {
    if (hasPendingUpdate && latestTokenState) {
      dispatch({ type: 'SET_TOKEN_STATE', payload: latestTokenState });
    }
    dispatch({ type: 'SET_MESSAGES', payload: [...currentMessages] });
    dispatch({ type: 'SET_CHAT_STATE', payload: latestChatState });
    hasPendingUpdate = false;
    lastBatchUpdate = Date.now();
  };

  const maybeUpdateUI = (tokenState: TokenState, chatState: ChatState, forceImmediate = false) => {
    latestTokenState = tokenState;
    latestChatState = chatState;

    if (!reduceMotion || forceImmediate) {
      dispatch({ type: 'SET_TOKEN_STATE', payload: tokenState });
      dispatch({ type: 'SET_MESSAGES', payload: [...currentMessages] });
      dispatch({ type: 'SET_CHAT_STATE', payload: chatState });
      lastBatchUpdate = Date.now();
    } else {
      hasPendingUpdate = true;
      const now = Date.now();
      if (now - lastBatchUpdate >= REDUCED_MOTION_BATCH_INTERVAL) {
        flushBatchedUpdates();
      }
    }
  };

  try {
    for await (const event of stream) {
      switch (event.type) {
        case 'Message': {
          const msg = event.message;
          if (msg.role === 'assistant') {
            if (currentModelInfo) {
              (msg as MessageWithAttribution)._modelInfo = { ...currentModelInfo };
            }
            if (currentRoutingInfo) {
              (msg as MessageWithAttribution)._routingInfo = { ...currentRoutingInfo };
            }
          }
          currentMessages = pushMessage(currentMessages, msg);

          const hasToolConfirmation = msg.content.some(
            (content) =>
              content.type === 'actionRequired' && content.data.actionType === 'toolConfirmation'
          );

          const hasElicitation = msg.content.some(
            (content) =>
              content.type === 'actionRequired' && content.data.actionType === 'elicitation'
          );

          if (hasToolConfirmation || hasElicitation) {
            maybeUpdateUI(event.token_state, ChatState.WaitingForUserInput, true);
          } else if (getCompactingMessage(msg)) {
            maybeUpdateUI(event.token_state, ChatState.Compacting);
          } else if (getThinkingMessage(msg)) {
            maybeUpdateUI(event.token_state, ChatState.Thinking);
          } else {
            maybeUpdateUI(event.token_state, ChatState.Streaming);
          }
          break;
        }
        case 'Error': {
          flushBatchedUpdates();
          onFinish(`Stream error: ${event.error}`);
          return;
        }
        case 'Finish': {
          flushBatchedUpdates();
          onFinish();
          return;
        }
        case 'ModelChange': {
          currentModelInfo = { model: event.model, mode: event.mode };
          break;
        }
        case 'RoutingDecision': {
          currentRoutingInfo = {
            agentName: event.agent_name,
            modeSlug: event.mode_slug,
            confidence: event.confidence,
            reasoning: event.reasoning,
          };
          break;
        }
        case 'UpdateConversation': {
          currentMessages = event.conversation;
          if (!reduceMotion) {
            dispatch({ type: 'SET_MESSAGES', payload: event.conversation });
          } else {
            hasPendingUpdate = true;
          }
          break;
        }
        case 'Notification': {
          dispatch({ type: 'ADD_NOTIFICATION', payload: event as NotificationEvent });
          maybeHandlePlatformEvent(event.message, sessionId);
          break;
        }
        case 'Ping':
          break;
      }
    }

    flushBatchedUpdates();
    onFinish();
  } catch (error) {
    flushBatchedUpdates();
    if (error instanceof Error && error.name !== 'AbortError') {
      onFinish(`Stream error: ${errorMessage(error)}`);
    }
  }
}
