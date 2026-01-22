import { AppEvents } from '../constants/events';
import { useCallback, useEffect, useRef } from 'react';
import { useSearchParams } from 'react-router-dom';
import { Session } from '../api';
import { Message } from '../api';
import { ChatState } from '../types/chatState';
import { ImageData } from '../types/message';

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
  initialMessage: { msg: string; images: ImageData[] } | undefined;
  handleSubmit: (message: string, images: ImageData[]) => void;
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
  const [searchParams] = useSearchParams();
  const hasAutoSubmittedRef = useRef(false);

  // Reset auto-submit flag when session changes
  useEffect(() => {
    hasAutoSubmittedRef.current = false;
  }, [sessionId]);

  const clearInitialMessage = useCallback(() => {
    window.dispatchEvent(
      new CustomEvent(AppEvents.CLEAR_INITIAL_MESSAGE, {
        detail: { sessionId },
      })
    );
  }, [sessionId]);

  // Auto-submit logic
  useEffect(() => {
    const currentSessionId = searchParams.get('resumeSessionId');
    const isCurrentSession = currentSessionId === sessionId;
    const shouldStartAgent = isCurrentSession && searchParams.get('shouldStartAgent') === 'true';

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
      handleSubmit(initialMessage.msg, initialMessage.images);
      clearInitialMessage();
      return;
    }

    // Scenario 2: Forked session with edited message
    if (shouldStartAgent && initialMessage) {
      hasAutoSubmittedRef.current = true;
      handleSubmit(initialMessage.msg, initialMessage.images);
      clearInitialMessage();
      return;
    }

    // Scenario 3: Resume with shouldStartAgent (continue existing conversation)
    if (shouldStartAgent) {
      hasAutoSubmittedRef.current = true;
      handleSubmit('', []);
    }
  }, [
    session,
    initialMessage,
    searchParams,
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
