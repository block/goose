import { useEffect, useState } from 'react';
import { ChatType } from '../types/chat';
import { fetchSessionDetails, generateSessionId } from '../sessions';
import { View, ViewOptions } from '../App';

type UseChatArgs = {
  setIsLoadingSession: (isLoading: boolean) => void;
  setView: (view: View, viewOptions?: ViewOptions) => void;
  setPairChat?: (chat: ChatType) => void;
};
export const useChat = ({ setIsLoadingSession, setView, setPairChat }: UseChatArgs) => {
  const [chat, setChat] = useState<ChatType>({
    id: generateSessionId(),
    title: 'New Chat',
    messages: [],
    messageHistoryIndex: 0,
    recipeConfig: null, // Initialize with no recipe
  });

  // Check for resumeSessionId in URL parameters
  useEffect(() => {
    const checkForResumeSession = async () => {
      const urlParams = new URLSearchParams(window.location.search);
      const resumeSessionId = urlParams.get('resumeSessionId');

      if (!resumeSessionId) {
        return;
      }

      setIsLoadingSession(true);
      try {
        const sessionDetails = await fetchSessionDetails(resumeSessionId);

        // Only set view if we have valid session details
        if (sessionDetails && sessionDetails.session_id) {
          const sessionChat = {
            id: sessionDetails.session_id,
            title: sessionDetails.metadata?.description || `ID: ${sessionDetails.session_id}`,
            messages: sessionDetails.messages,
            messageHistoryIndex: sessionDetails.messages.length,
            recipeConfig: null, // Sessions don't have recipes by default
          };

          setChat(sessionChat);

          // If we're setting the view to 'pair', also update the pairChat state
          if (setPairChat) {
            setPairChat(sessionChat);
          }

          setView('pair');
        } else {
          console.error('Invalid session details received');
        }
      } catch (error) {
        console.error('Failed to fetch session details:', error);
      } finally {
        // Always clear the loading state
        setIsLoadingSession(false);
      }
    };

    checkForResumeSession();
    // todo: rework this to allow for exhaustive deps currently throws app in error loop
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return { chat, setChat };
};
