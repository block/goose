import { useEffect } from 'react';
import { generateSessionId } from '../sessions';
import { ChatType } from '../components/hub';

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
  // Handle session continuation when there's a summarized thread
  useEffect(() => {
    // If we're in a continuation session, update the chat ID
    if (summarizedThread.length > 0) {
      const newSessionId = generateSessionId();

      // Update the session ID in the chat object
      setChat({
        ...chat,
        id: newSessionId!,
        title: `Continued from ${chat.id}`,
        messageHistoryIndex: summarizedThread.length,
      });

      // Update the body used by useMessageStream to send future messages to the new session
      if (summarizedThread.length > 0 && updateMessageStreamBody) {
        updateMessageStreamBody({
          session_id: newSessionId,
          session_working_dir: window.appConfig.get('GOOSE_WORKING_DIR'),
        });
      }
    }

    // only update if summarizedThread length changes from 0 -> 1+
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    // eslint-disable-next-line react-hooks/exhaustive-deps
    summarizedThread.length > 0,
  ]);
};
