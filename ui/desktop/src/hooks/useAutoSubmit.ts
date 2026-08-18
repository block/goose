import { AppEvents } from '../constants/events';
import { useCallback, useEffect, useRef } from 'react';
import { useSearchParams } from 'react-router';
import { ChatState } from '../types/chatState';
import type { Message, UserInput } from '../types/message';
import type { Session } from '../types/session';

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
  canAutoSubmit?: boolean;
  /** Return false (or Promise<false>) when submit was skipped so we can retry. */
  handleSubmit: (input: UserInput) => boolean | void | Promise<boolean | void>;
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
  canAutoSubmit = true,
  handleSubmit,
}: UseAutoSubmitProps): UseAutoSubmitReturn {
  const [searchParams] = useSearchParams();
  const hasAutoSubmittedRef = useRef(false);
  const inFlightRef = useRef(false);

  // Reset auto-submit flag when session changes
  useEffect(() => {
    hasAutoSubmittedRef.current = false;
    inFlightRef.current = false;
  }, [sessionId]);

  const clearInitialMessage = useCallback(() => {
    window.dispatchEvent(
      new CustomEvent(AppEvents.CLEAR_INITIAL_MESSAGE, {
        detail: { sessionId },
      })
    );
  }, [sessionId]);

  const hasUnfilledParameters = useCallback((session: Session) => {
    if (session.session_type === 'scheduled') {
      return false;
    }

    const recipe = session.recipe;
    return recipe?.parameters && recipe.parameters.length > 0 && !session.user_recipe_values;
  }, []);

  const trySubmit = useCallback(
    async (input: UserInput, clearMessage: boolean) => {
      if (inFlightRef.current || hasAutoSubmittedRef.current) {
        return;
      }

      inFlightRef.current = true;
      try {
        const accepted = await Promise.resolve(handleSubmit(input));
        // void/undefined from legacy callers counts as accepted
        if (accepted === false) {
          return;
        }
        hasAutoSubmittedRef.current = true;
        if (clearMessage) {
          clearInitialMessage();
        }
      } finally {
        inFlightRef.current = false;
      }
    },
    [clearInitialMessage, handleSubmit]
  );

  // Auto-submit logic
  useEffect(() => {
    const currentSessionId = searchParams.get('resumeSessionId');
    const isCurrentSession = currentSessionId === sessionId;
    const shouldStartAgent = isCurrentSession && searchParams.get('shouldStartAgent') === 'true';

    if (!session || hasAutoSubmittedRef.current || inFlightRef.current) {
      return;
    }

    if (!canAutoSubmit) {
      return;
    }

    if (chatState !== ChatState.Idle) {
      return;
    }

    // Scenario 1: New session with initial message from Hub
    // Hub always creates new sessions, so message_count will be 0
    if (initialMessage && session.message_count === 0 && messages.length === 0) {
      if (!hasUnfilledParameters(session)) {
        void trySubmit(initialMessage, true);
      }
      return;
    }

    // Scenario 2: Forked session with edited message
    if (shouldStartAgent && initialMessage) {
      if (messages.length > 0) {
        void trySubmit(initialMessage, true);
        return;
      }
      return;
    }

    // Scenario 3: Resume with shouldStartAgent (continue existing conversation)
    if (shouldStartAgent) {
      if (!hasUnfilledParameters(session)) {
        void trySubmit({ msg: '', images: [] }, false);
      }
      return;
    }
  }, [
    session,
    initialMessage,
    searchParams,
    trySubmit,
    sessionId,
    messages.length,
    chatState,
    canAutoSubmit,
    hasUnfilledParameters,
  ]);

  return {
    hasAutoSubmitted: hasAutoSubmittedRef.current,
  };
}
