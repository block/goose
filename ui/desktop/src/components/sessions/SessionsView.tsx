import React, { useState, useEffect, useCallback } from 'react';
import { History, NotebookPen } from 'lucide-react';
import { View, ViewOptions } from '../../App';
import { fetchSessionDetails, type SessionDetails } from '../../sessions';
import SessionListView from './SessionListView';
import { DraftsViewContent } from '../drafts/DraftsView';
import SessionHistoryView from './SessionHistoryView';
import { MainPanelLayout } from '../Layout/MainPanelLayout';
import { toastError } from '../../toasts';
import { useLocation } from 'react-router-dom';

interface SessionsViewProps {
  setView: (view: View, viewOptions?: ViewOptions) => void;
}

type TabType = 'history' | 'drafts';

const SessionsView: React.FC<SessionsViewProps> = ({ setView }) => {
  const [selectedSession, setSelectedSession] = useState<SessionDetails | null>(null);
  const [isLoadingSession, setIsLoadingSession] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [initialSessionId, setInitialSessionId] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<TabType>('history');
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

  const tabs = [
    {
      id: 'history' as TabType,
      label: 'All history',
      icon: History,
    },
    {
      id: 'drafts' as TabType,
      label: 'Drafts',
      icon: NotebookPen,
    },
  ];

  // If we're loading an initial session or have a selected session, show the session history view
  // Otherwise, show the sessions list view with tabs
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
    <MainPanelLayout>
      <div className="flex-1 flex flex-col min-h-0">
        {/* Header with tabs */}
        <div className="bg-background-default px-8 pb-6 pt-16">
          <div className="flex flex-col page-transition">
            <h1 className="text-4xl font-light mb-1">History</h1>
            <p className="text-sm text-text-muted mb-6">
              View and search your past conversations with goose
            </p>

            {/* Tab buttons */}
            <div className="flex gap-6">
              {tabs.map((tab) => {
                const Icon = tab.icon;
                return (
                  <button
                    key={tab.id}
                    onClick={() => setActiveTab(tab.id)}
                    className={`flex items-center gap-2 pb-3 px-1 border-b-2 transition-all duration-200 ${
                      activeTab === tab.id
                        ? 'border-textStandard text-textStandard'
                        : 'border-transparent text-textSubtle hover:text-textStandard'
                    }`}
                  >
                    <Icon className="w-4 h-4" />
                    <span className="text-sm font-medium">{tab.label}</span>
                  </button>
                );
              })}
            </div>
          </div>
        </div>

        {/* Tab content */}
        <div className="flex-1 min-h-0">
          {activeTab === 'history' ? (
            <SessionListView setView={setView} onSelectSession={handleSelectSession} />
          ) : (
            <DraftsViewContent />
          )}
        </div>
      </div>
    </MainPanelLayout>
  );
};

export default SessionsView;
