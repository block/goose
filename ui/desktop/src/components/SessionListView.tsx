import React, { useEffect, useState } from 'react';
import { ViewConfig } from '../App';
import { ArrowLeft, Clock, RefreshCw } from 'lucide-react';
import { fetchSessions, type Session } from '../api/sessions';
import { Card } from './ui/card';
import { Button } from './ui/button';

interface SessionListViewProps {
  setView: (view: ViewConfig['view'], viewOptions?: Record<any, any>) => void;
  onSelectSession: (sessionId: string) => void;
}

const SessionListView: React.FC<SessionListViewProps> = ({ setView, onSelectSession }) => {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Load sessions on component mount
    loadSessions();
  }, []);

  const loadSessions = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await fetchSessions();
      setSessions(response.sessions);
    } catch (err) {
      console.error('Failed to load sessions:', err);
      setError('Failed to load sessions. Please try again later.');
      setSessions([]);
    } finally {
      setIsLoading(false);
    }
  };

  // Format date to be more readable
  const formatDate = (dateString: string) => {
    try {
      const date = new Date(dateString);
      return new Intl.DateTimeFormat('en-US', {
        month: 'short',
        day: 'numeric',
        year: 'numeric',
        hour: 'numeric',
        minute: 'numeric',
      }).format(date);
    } catch (e) {
      return dateString;
    }
  };

  return (
    <div className="flex flex-col h-screen bg-bgApp">
      {/* Header */}
      <div className="flex items-center p-4 border-b border-borderSubtle">
        <button
          onClick={() => setView('chat')}
          className="flex items-center text-textPrimary hover:text-textSecondary transition-colors"
          aria-label="Back to chat"
        >
          <ArrowLeft className="w-5 h-5 mr-2" />
          Back to chat
        </button>
        <h1 className="text-xl font-semibold text-textPrimary ml-4">Chat History</h1>

        {/* Refresh button */}
        {!isLoading && (
          <button
            onClick={loadSessions}
            className="ml-auto flex items-center text-textSubtle hover:text-textPrimary transition-colors"
            aria-label="Refresh sessions"
          >
            <RefreshCw className={`w-4 h-4 mr-1 ${isLoading ? 'animate-spin' : ''}`} />
            <span>Refresh</span>
          </button>
        )}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-4">
        {isLoading ? (
          <div className="flex justify-center items-center h-full">
            <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-textPrimary"></div>
          </div>
        ) : error ? (
          <div className="flex flex-col items-center justify-center h-full text-textSubtle">
            <div className="text-red-500 mb-4">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="48"
                height="48"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <circle cx="12" cy="12" r="10"></circle>
                <line x1="12" y1="8" x2="12" y2="12"></line>
                <line x1="12" y1="16" x2="12.01" y2="16"></line>
              </svg>
            </div>
            <p className="text-lg mb-2">Error Loading Sessions</p>
            <p className="text-sm text-center mb-4">{error}</p>
            <Button onClick={loadSessions} variant="default">
              Try Again
            </Button>
          </div>
        ) : sessions.length > 0 ? (
          <div className="grid gap-4">
            {sessions.map((session) => (
              <Card
                key={session.id}
                onClick={() => onSelectSession(session.id)}
                className="p-4 bg-bgSecondary border border-borderSubtle hover:border-borderPrimary cursor-pointer transition-all"
              >
                <div className="flex justify-between items-start">
                  <h3 className="text-lg font-medium text-textPrimary">
                    {session.description || session.id}
                  </h3>
                  <div className="flex items-center text-sm text-textSubtle">
                    <Clock className="w-4 h-4 mr-1" />
                    <span>{formatDate(session.modified)}</span>
                  </div>
                </div>
                <div className="flex items-center mt-2 text-textSubtle text-sm truncate">
                  <span className="truncate max-w-[300px]">{session.path}</span>
                </div>
              </Card>
            ))}
          </div>
        ) : (
          <div className="flex flex-col items-center justify-center h-full text-textSubtle">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="48"
              height="48"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              className="mb-4"
            >
              <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
            </svg>
            <p className="text-lg mb-2">No chat sessions found</p>
            <p className="text-sm">Your chat history will appear here</p>
          </div>
        )}
      </div>
    </div>
  );
};

export default SessionListView;
