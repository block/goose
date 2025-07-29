import { useEffect, useRef } from 'react';
import { generateSessionId } from '../sessions';
import { ChatType } from '../types/chat';

interface UseSessionContinuationProps {
  chat: ChatType;
  setChat: (chat: ChatType) => void;
  summarizedThread: unknown[];
  updateMessageStreamBody?: (body: Record<string, unknown>) => void;
}

export const useSessionContinuation = ({
  chat,
  setChat,
  summarizedThread,
  updateMessageStreamBody,
}: UseSessionContinuationProps) => {
  // Track if we've already created a new session to prevent multiple calls
  const hasCreatedNewSession = useRef(false);
  // Track the previous summarized thread length to detect when it gets cleared
  const prevSummarizedThreadLength = useRef(0);
  // Flag to indicate we should create a new session after the next message is sent
  const shouldCreateNewSessionAfterMessage = useRef(false);

  // Handle session continuation when there's a summarized thread
  useEffect(() => {
    // Detect when summarizedThread goes from having content to being empty
    // This indicates the summary process is complete and messages have been reset
    const wasCleared = prevSummarizedThreadLength.current > 0 && summarizedThread.length === 0;

    // When the thread is cleared, mark that we should create a new session
    // but don't do it immediately - wait for the message to be sent first
    if (wasCleared && !hasCreatedNewSession.current) {
      shouldCreateNewSessionAfterMessage.current = true;
    }

    // Update the previous length for next comparison
    prevSummarizedThreadLength.current = summarizedThread.length;

    // Reset the flag when we start a completely new session (no summarized thread at all)
    if (summarizedThread.length === 0 && prevSummarizedThreadLength.current === 0) {
      hasCreatedNewSession.current = false;
      shouldCreateNewSessionAfterMessage.current = false;
    }
  }, [summarizedThread.length]);

  // Function to be called after a message is successfully sent
  const createNewSessionIfNeeded = () => {
    if (shouldCreateNewSessionAfterMessage.current && !hasCreatedNewSession.current) {
      const newSessionId = generateSessionId();

      // Mark that we've created a new session
      hasCreatedNewSession.current = true;
      shouldCreateNewSessionAfterMessage.current = false;

      console.log('Creating new session, preserving recipe config:', {
        oldSessionId: chat.id,
        newSessionId,
        currentRecipeConfig: chat.recipeConfig,
        recipeTitle: chat.recipeConfig?.title,
      });

      // Update the session ID in the chat object while preserving recipe config
      setChat({
        ...chat,
        id: newSessionId!,
        title: `Continued from ${chat.id}`,
        messageHistoryIndex: 0, // Reset since messages were already reset
        // Explicitly preserve the recipe config
        recipeConfig: chat.recipeConfig,
      });

      // Update the body used by useMessageStream to send future messages to the new session
      if (updateMessageStreamBody) {
        updateMessageStreamBody({
          session_id: newSessionId,
          session_working_dir: window.appConfig.get('GOOSE_WORKING_DIR'),
        });
      }
    }
  };

  return { createNewSessionIfNeeded };
};
