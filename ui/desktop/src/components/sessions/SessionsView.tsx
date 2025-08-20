import React, { useState, useEffect, useCallback } from 'react';
import { View, ViewOptions } from '../../utils/navigationUtils';
import { fetchSessionDetails, type SessionDetails } from '../../sessions';
import SessionListView from './SessionListView';
import SessionHistoryView from './SessionHistoryView';
import { toastError } from '../../toasts';
import { useLocation } from 'react-router-dom';
import BackgroundImageFix from '../BackgroundImageFix';

interface SessionsViewProps {
  setView: (view: View, viewOptions?: ViewOptions) => void;
}

const SessionsView: React.FC<SessionsViewProps> = ({ setView }) => {
  const [selectedSession, setSelectedSession] = useState<SessionDetails | null>(null);
  const [isLoadingSession, setIsLoadingSession] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [initialSessionId, setInitialSessionId] = useState<string | null>(null);
  const location = useLocation();

  // Use BackgroundImageFix for consistent background across the app
  useEffect(() => {
    // Override any background colors that might be covering our background
    const mainPanels = document.querySelectorAll('.bg-background-default, .bg-background-muted') as NodeListOf<HTMLElement>;
    mainPanels.forEach(panel => {
      if (panel) {
        panel.style.background = 'transparent';
        panel.style.backgroundColor = 'transparent';
      }
    });

    // Make session list transparent
    const sessionList = document.querySelector('.sessions-list-container') as HTMLElement;
    if (sessionList) {
      sessionList.style.background = 'transparent';
      sessionList.style.backgroundColor = 'transparent';
    }

    // Make session items transparent but with glass effect
    const sessionItems = document.querySelectorAll('.session-item') as NodeListOf<HTMLElement>;
    sessionItems.forEach(item => {
      if (item) {
        item.style.backgroundColor = 'rgba(255, 255, 255, 0.1)';
        item.style.backdropFilter = 'blur(10px)';
      }
    });

    return () => {
      // Cleanup styles
      mainPanels.forEach(panel => {
        if (panel) {
          panel.style.background = '';
          panel.style.backgroundColor = '';
        }
      });
      
      if (sessionList) {
        sessionList.style.background = '';
        sessionList.style.backgroundColor = '';
      }
      
      sessionItems.forEach(item => {
        if (item) {
          item.style.backgroundColor = '';
          item.style.backdropFilter = '';
        }
      });
    };
  }, []);

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

  return (
    <div className="relative h-full">
      {/* Use the global BackgroundImageFix component */}
      <BackgroundImageFix />
      
      {/* Content layer with transparent background */}
      <div className="relative z-10 h-full bg-transparent">
        {selectedSession || (isLoadingSession && initialSessionId) ? (
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
        )}
      </div>
    </div>
  );
};

export default SessionsView;
