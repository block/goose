import { useEffect, useState } from 'react';
import { ChatType } from '../components/ChatView';
import { fetchSessionDetails, generateSessionId } from '../sessions';
import { View } from '../types/views';
import { APISessionResponse, SessionDetails } from '../types/sessions';

function mapAPISessionToDetails(apiSession: APISessionResponse): SessionDetails {
  return {
    id: apiSession.session_id,
    path: '',
    created: new Date().toISOString(),
    modified: new Date().toISOString(),
    metadata: apiSession.metadata || {
      message_count: apiSession.messages.length,
      total_tokens: null,
    },
    messages: apiSession.messages,
  };
}

type UseChatArgs = {
  setIsLoadingSession: (isLoading: boolean) => void;
  setView: (view: View) => void;
};

export const useChat = ({ setIsLoadingSession, setView }: UseChatArgs) => {
  const [chat, setChat] = useState<ChatType>({
    id: generateSessionId(),
    title: 'New Chat',
    messages: [],
    messageHistoryIndex: 0,
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
        const apiResponse = await fetchSessionDetails(resumeSessionId);

        // Only set view if we have valid session details
        if (apiResponse && apiResponse.session_id) {
          const sessionDetails = mapAPISessionToDetails(apiResponse);
          setChat({
            id: sessionDetails.id,
            title: sessionDetails.metadata?.description || `ID: ${sessionDetails.id}`,
            messages: sessionDetails.messages,
            messageHistoryIndex: sessionDetails.messages.length,
          });
          setView('chat');
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
  }, [setIsLoadingSession, setView]);

  return { chat, setChat };
};
