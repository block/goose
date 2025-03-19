import React, { useState } from 'react';
import { ViewConfig } from '../../App';
import { fetchSessionDetails, type SessionDetails } from '../../sessions';
import { fetchSharedSessionDetails } from '../../sharedSessions';
import SessionListView from './SessionListView';
import SessionHistoryView from './SessionHistoryView';
import { Card } from '../ui/card';
import { Input } from '../ui/input';
import { Button } from '../ui/button';
import BackButton from '../ui/BackButton';
import { ScrollArea } from '../ui/scroll-area';

interface SessionsViewProps {
  setView: (view: ViewConfig['view'], viewOptions?: Record<any, any>) => void;
}

const SessionsView: React.FC<SessionsViewProps> = ({ setView }) => {
  const [selectedSession, setSelectedSession] = useState<SessionDetails | null>(null);
  const [isLoadingSession, setIsLoadingSession] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showSharedSessionTest, setShowSharedSessionTest] = useState(false);
  const [shareToken, setShareToken] = useState('6dc69efc-5599-4493-968e-8615a93dd780');
  const [baseUrl, setBaseUrl] = useState(
    'https://goosed-playpen-smohammed--goosed.stage.sqprod.co/api'
  );

  const handleSelectSession = async (sessionId: string) => {
    await loadSessionDetails(sessionId);
  };

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
    } finally {
      setIsLoadingSession(false);
    }
  };

  const handleBackToSessions = () => {
    setSelectedSession(null);
    setError(null);
    setShowSharedSessionTest(false);
  };

  const handleResumeSession = () => {
    if (selectedSession) {
      // Get the working directory from the session metadata
      const workingDir = selectedSession.metadata.working_dir;

      if (workingDir) {
        console.log(
          `Resuming session with ID: ${selectedSession.session_id}, in working dir: ${workingDir}`
        );

        // Create a new chat window with the working directory and session ID
        window.electron.createChatWindow(
          undefined,
          workingDir,
          undefined,
          selectedSession.session_id
        );
      } else {
        // Fallback if no working directory is found
        console.error('No working directory found in session metadata');
        // We could show a toast or alert here
      }
    }
  };

  const handleRetryLoadSession = () => {
    if (selectedSession) {
      loadSessionDetails(selectedSession.session_id);
    }
  };

  const handleTestSharedSession = async () => {
    if (!shareToken || !baseUrl) {
      setError('Please enter both a share token and base URL');
      return;
    }

    setIsLoadingSession(true);
    setError(null);

    try {
      // Get the saved base URL from localStorage if available
      let finalBaseUrl = baseUrl;
      if (!finalBaseUrl) {
        const savedSessionConfig = localStorage.getItem('session_sharing_config');
        if (savedSessionConfig) {
          const config = JSON.parse(savedSessionConfig);
          if (config.enabled && config.baseUrl) {
            finalBaseUrl = config.baseUrl;
          }
        }
      }

      if (!finalBaseUrl) {
        throw new Error('Base URL is not configured. Please enter a base URL.');
      }

      // Fetch the shared session details
      const sessionDetails = await fetchSharedSessionDetails(finalBaseUrl, shareToken);

      // Navigate to the shared session view
      setView('sharedSession', {
        sessionDetails,
        shareToken,
        baseUrl: finalBaseUrl,
      });
    } catch (error) {
      console.error('Failed to load shared session:', error);
      setError(
        `Failed to load shared session: ${error instanceof Error ? error.message : 'Unknown error'}`
      );
    } finally {
      setIsLoadingSession(false);
    }
  };

  // If showing shared session test interface
  if (showSharedSessionTest) {
    return (
      <div className="h-screen w-full">
        <div className="relative flex items-center h-[36px] w-full bg-bgSubtle"></div>

        <Card className="px-8 pt-6 pb-4 bg-bgSecondary">
          <div className="flex items-center">
            <BackButton onClick={handleBackToSessions} />
            <h1 className="text-3xl font-medium text-textStandard ml-2">Test Shared Session</h1>
          </div>
        </Card>

        <ScrollArea className="h-[calc(100vh-120px)] w-full">
          <div className="p-8">
            <Card className="p-6 bg-bgSecondary">
              <h2 className="text-xl font-semibold mb-4">Enter Shared Session Details</h2>

              <div className="space-y-4">
                <div>
                  <label className="block text-sm font-medium mb-1">Base URL</label>
                  <Input
                    value={baseUrl}
                    onChange={(e) => setBaseUrl(e.target.value)}
                    className="w-full"
                  />
                  <p className="text-xs text-textSubtle mt-1">
                    Leave empty to use the configured base URL from settings
                  </p>
                </div>

                <div>
                  <label className="block text-sm font-medium mb-1">Share Token</label>
                  <Input
                    value={shareToken}
                    onChange={(e) => setShareToken(e.target.value)}
                    className="w-full"
                  />
                </div>

                {error && <div className="text-red-500 text-sm mt-2">{error}</div>}

                <Button
                  onClick={handleTestSharedSession}
                  disabled={isLoadingSession}
                  className="mt-4"
                >
                  {isLoadingSession ? 'Loading...' : 'Load Shared Session'}
                </Button>
              </div>
            </Card>
          </div>
        </ScrollArea>
      </div>
    );
  }

  // If a session is selected, show the session history view
  // Otherwise, show the sessions list view with a button to test shared sessions
  return selectedSession ? (
    <SessionHistoryView
      session={selectedSession}
      isLoading={isLoadingSession}
      error={error}
      onBack={handleBackToSessions}
      onResume={handleResumeSession}
      onRetry={handleRetryLoadSession}
    />
  ) : (
    <>
      <SessionListView setView={setView} onSelectSession={handleSelectSession} />
      <div className="fixed bottom-4 right-4 text-textStandard">
        <Button variant="outline" onClick={() => setShowSharedSessionTest(true)}>
          Test Shared Session
        </Button>
      </div>
    </>
  );
};

export default SessionsView;
