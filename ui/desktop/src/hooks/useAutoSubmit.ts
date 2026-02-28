import { useCallback, useEffect, useRef } from 'react';
import { useLocation } from 'react-router-dom';

import type { Message, Session } from '@/api';
import { AppEvents } from '@/constants/events';
import { ChatState } from '@/types/chatState';
import type { UserInput } from '@/types/message';

const getSessionIdFromPath = (pathname: string): string | undefined => {
  const match = pathname.match(/^\/sessions\/([^/]+)$/);
  if (!match) return undefined;
  try {
    return decodeURIComponent(match[1]);
  } catch {
    return match[1];
  }
};

/**
 * Auto-submit scenarios:
 * 1. New session with initial message from Hub (message_count === 0, has initialMessage)
 * 2. Forked session with edited message (shouldStartAgent + initialMessage)
 * 3. Resume with shouldStartAgent (continue existing conversation)
 */

interface UseAutoSubmitProps {
  sessionId: string;
  session: Session | undefined;
  messages: Message[];
  chatState: ChatState;
  initialMessage: UserInput | undefined;
  handleSubmit: (input: UserInput) => void;
}

interface UseAutoSubmitReturn {
  hasAutoSubmitted: boolean;
}

export function useAutoSubmit({
  sessionId,
  session,
  messages,
  chatState,
  initialMessage,
  handleSubmit,
}: UseAutoSubmitProps): UseAutoSubmitReturn {
  const location = useLocation();
  const hasAutoSubmittedRef = useRef(false);

  // Reset auto-submit flag when session changes
  useEffect(() => {
    hasAutoSubmittedRef.current = false;
  }, []);

  const clearInitialMessage = useCallback(() => {
    window.dispatchEvent(
      new CustomEvent(AppEvents.CLEAR_INITIAL_MESSAGE, {
        detail: { sessionId },
      })
    );
  }, [sessionId]);

  // Auto-submit logic
  useEffect(() => {
    const currentSessionId = getSessionIdFromPath(location.pathname);
    const isCurrentSession = currentSessionId === sessionId;
    const shouldStartAgent = isCurrentSession && location.state?.shouldStartAgent === true;

    if (!session || hasAutoSubmittedRef.current) {
      return;
    }

    // Don't submit if already streaming or loading
    if (chatState !== ChatState.Idle) {
      return;
    }

    // Scenario 1: New session with initial message from Hub
    // Hub always creates new sessions, so message_count will be 0
    if (initialMessage && session.message_count === 0 && messages.length === 0) {
      hasAutoSubmittedRef.current = true;
      handleSubmit(initialMessage);
      clearInitialMessage();
      return;
    }

    // Scenario 2: Forked session with edited message
    if (shouldStartAgent && initialMessage) {
      if (messages.length > 0) {
        hasAutoSubmittedRef.current = true;
        handleSubmit(initialMessage);
        clearInitialMessage();
        return;
      }
      return;
    }

    // Scenario 3: Resume with shouldStartAgent (continue existing conversation)
    if (shouldStartAgent) {
      hasAutoSubmittedRef.current = true;
      handleSubmit({ msg: '', images: [] });
    }
  }, [
    session,
    initialMessage,
    location.pathname,
    location.state,
    handleSubmit,
    sessionId,
    messages.length,
    chatState,
    clearInitialMessage,
  ]);

  return {
    hasAutoSubmitted: hasAutoSubmittedRef.current,
  };
}
