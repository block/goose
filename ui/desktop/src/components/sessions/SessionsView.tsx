import React, { useState, useRef } from 'react';
import { ViewConfig } from '../../App';
import { fetchSessionDetails, type SessionDetails } from '../../sessions';
import { importSessionFromFile } from '../../sessionFiles';
import SessionListView from './SessionListView';
import SessionHistoryView from './SessionHistoryView';
import { Button } from '../ui/button';
import { toast } from 'react-toastify';

interface SessionsViewProps {
  setView: (view: ViewConfig['view'], viewOptions?: Record<any, any>) => void;
}

const SessionsView: React.FC<SessionsViewProps> = ({ setView }) => {
  const [selectedSession, setSelectedSession] = useState<SessionDetails | null>(null);
  const [isLoadingSession, setIsLoadingSession] = useState(false);
  const [error, setError] = useState<string | null>(null);

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

  // File input reference for importing sessions
  const fileInputRef = useRef<HTMLInputElement>(null);

  // Handle file import
  const handleImportSession = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    try {
      const importedSession = await importSessionFromFile(file);
      setSelectedSession(importedSession);
      toast.success('Session imported successfully!');
    } catch (error) {
      console.error('Failed to import session:', error);
      toast.error(
        `Failed to import session: ${error instanceof Error ? error.message : 'Unknown error'}`
      );
    } finally {
      // Reset the file input
      if (fileInputRef.current) {
        fileInputRef.current.value = '';
      }
    }
  };

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

      {/* Hidden file input for importing sessions */}
      <input
        type="file"
        ref={fileInputRef}
        onChange={handleImportSession}
        accept=".egg"
        style={{ display: 'none' }}
      />

      {/* Import button */}
      <div className="fixed bottom-8 right-8">
        <Button
          onClick={() => fileInputRef.current?.click()}
          className="bg-indigo-500 hover:bg-indigo-600 text-white"
        >
          Import from File
        </Button>
      </div>
    </>
  );
};

export default SessionsView;
