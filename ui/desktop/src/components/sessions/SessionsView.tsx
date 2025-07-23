import React, { useState, useEffect, useCallback } from 'react';
import { View, ViewOptions } from '../../App';
import { fetchSessionDetails, type SessionDetails } from '../../sessions';
import SessionListView from './SessionListView';
import SessionHistoryView from './SessionHistoryView';
import { toastError } from '../../toasts';
import { useLocation } from 'react-router-dom';

interface SessionsViewProps {
  setView: (view: View, viewOptions?: ViewOptions) => void;
}

const SessionsView: React.FC<SessionsViewProps> = ({ setView }) => {
  const [selectedSession, setSelectedSession] = useState<SessionDetails | null>(null);
  const [isLoadingSession, setIsLoadingSession] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [initialSessionId, setInitialSessionId] = useState<string | null>(null);
  const location = useLocation();

  const loadSessionDetails = async (sessionId: string) => {
    setIsLoadingSession(true);
    setError(null);
    try {
      const sessionDetails = await fetchSessionDetails(sessionId);
      setSelectedSession(sessionDetails);
    } catch (err) {
      console.error(`Failed to load session details for ${sessionId}:`, err);
      setError('Failed to load session details. Please try again later.');
      // Keep the selected session null if there's an error
      setSelectedSession(null);

      const errorMessage = err instanceof Error ? err.message : String(err);
      toastError({
        title: 'Failed to load session. The file may be corrupted.',
        msg: 'Please try again later.',
        traceback: errorMessage,
      });
    } finally {
      setIsLoadingSession(false);
      setInitialSessionId(null);
    }
  };

  const handleSelectSession = useCallback(async (sessionId: string) => {
    await loadSessionDetails(sessionId);
  }, []);

  // Check if a session ID was passed in the location state (from SessionsInsights)
  useEffect(() => {
    const state = location.state as { selectedSessionId?: string } | null;
    if (state?.selectedSessionId) {
      // Set immediate loading state to prevent flash of session list
      setIsLoadingSession(true);
      setInitialSessionId(state.selectedSessionId);
      handleSelectSession(state.selectedSessionId);
      // Clear the state to prevent reloading on navigation
      window.history.replaceState({}, document.title);
    }
  }, [location.state, handleSelectSession]);

  const handleBackToSessions = () => {
    setSelectedSession(null);
    setError(null);
  };

  const handleRetryLoadSession = () => {
    if (selectedSession) {
      loadSessionDetails(selectedSession.session_id);
    }
  };

  // If we're loading an initial session or have a selected session, show the session history view
  // Otherwise, show the sessions list view
  return selectedSession || (isLoadingSession && initialSessionId) ? (
    <SessionHistoryView
      session={
        selectedSession || {
          session_id: initialSessionId || '',
          messages: [],
          metadata: {
            description: 'Loading...',
            working_dir: '',
            message_count: 0,
            total_tokens: 0,
          },
        }
      }
      isLoading={isLoadingSession}
      error={error}
      onBack={handleBackToSessions}
      onRetry={handleRetryLoadSession}
    />
  ) : (
    <SessionListView setView={setView} onSelectSession={handleSelectSession} />
  );
};

export default SessionsView;
