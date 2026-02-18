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
 * 4. Recipe with prompt (recipe.prompt exists, no messages, parameters filled or no parameters)
 */

interface UseAutoSubmitProps {
  sessionId: string;
  session: Session | undefined;
  messages: Message[];
  chatState: ChatState;
  initialMessage: UserInput | undefined;
  handleSubmit: (input: UserInput) => void;
  recipeAccepted?: boolean;
  recipePrompt?: string;
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
  recipeAccepted = true,
  recipePrompt = '',
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
      const recipe = session.recipe;
      const hasUnfilledParameters =
        recipe?.parameters && recipe.parameters.length > 0 && !session.user_recipe_values;

      if (!hasUnfilledParameters) {
        hasAutoSubmittedRef.current = true;
        handleSubmit({ msg: '', images: [] });
      }
      return;
    }

    // Scenario 4: Recipe started from RecipesView
    const recipe = session.recipe;
    const hasMessages = (session.conversation?.length ?? 0) > 0;

    if (recipe && recipe.prompt && recipeAccepted && !hasMessages && !initialMessage) {
      const hasParameters = recipe.parameters && recipe.parameters.length > 0;
      const parametersAreFilled =
        session.user_recipe_values && Object.keys(session.user_recipe_values).length > 0;

      if (!hasParameters || parametersAreFilled) {
        hasAutoSubmittedRef.current = true;
        handleSubmit({ msg: recipePrompt, images: [] });
      }
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
    recipeAccepted,
    recipePrompt,
  ]);

  return {
    hasAutoSubmitted: hasAutoSubmittedRef.current,
  };
}
