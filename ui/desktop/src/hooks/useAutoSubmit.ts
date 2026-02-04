import { AppEvents } from '../constants/events';
import { useCallback, useEffect, useRef } from 'react';
import { useSearchParams } from 'react-router-dom';
import { Session } from '../api';
import { Message } from '../api';
import { ChatState } from '../types/chatState';
import { UserInput } from '../types/message';

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

    // Debug logging for fork scenario
    if (shouldStartAgent && initialMessage) {
      console.log('[useAutoSubmit] Fork scenario check:', {
        sessionId,
        hasSession: !!session,
        hasAutoSubmitted: hasAutoSubmittedRef.current,
        chatState,
        messagesLength: messages.length,
        hasInitialMessage: !!initialMessage,
        shouldStartAgent,
      });
    }

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
    // Wait for messages to load before auto-submitting to ensure user sees history
    if (shouldStartAgent && initialMessage) {
      // Only submit if the forked conversation has loaded (messages.length > 0)
      if (messages.length > 0) {
        console.log('[useAutoSubmit] Forked session: Auto-submitting with', {
          messageText: initialMessage.msg,
          messagesLength: messages.length,
        });
        hasAutoSubmittedRef.current = true;
        handleSubmit(initialMessage);
        clearInitialMessage();
        return;
      }
      console.log('[useAutoSubmit] Forked session: Waiting for messages to load');
      // If messages haven't loaded yet, wait for next render
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
