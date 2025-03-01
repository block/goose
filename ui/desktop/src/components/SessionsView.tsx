import React, { useEffect, useState } from 'react';
import { ViewConfig } from '../App';
import { ArrowLeft, Clock, MessageSquare, ExternalLink } from 'lucide-react';
import {
  fetchSessions,
  fetchSessionDetails,
  type Session,
  type SessionDetails,
} from '../api/sessions';

interface SessionsViewProps {
  setView: (view: ViewConfig['view'], viewOptions?: Record<any, any>) => void;
}

const SessionsView: React.FC<SessionsViewProps> = ({ setView }) => {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [selectedSession, setSelectedSession] = useState<SessionDetails | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isLoadingSession, setIsLoadingSession] = useState(false);
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
      // Clear any existing sessions when there's an error
      setSessions([]);
    } finally {
      setIsLoading(false);
    }
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

  const handleBackClick = () => {
    if (selectedSession) {
      // If viewing a session, go back to the sessions list
      setSelectedSession(null);
    } else {
      // If viewing the sessions list, go back to chat
      setView('chat');
    }
  };

  const handleSessionClick = (sessionId: string) => {
    loadSessionDetails(sessionId);
  };

  const handleResumeSession = () => {
    if (selectedSession) {
      // In a real implementation, you would pass the session messages to the ChatView
      setView('chat', {
        resumedSession: selectedSession,
      });
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

  // Extract a preview from the first user message
  const getSessionPreview = (session: SessionDetails): string => {
    const firstUserMessage = session.messages.find((msg) => msg.role === 'user');
    if (firstUserMessage && firstUserMessage.content[0]?.text) {
      return (
        firstUserMessage.content[0].text.substring(0, 100) +
        (firstUserMessage.content[0].text.length > 100 ? '...' : '')
      );
    }
    return 'No preview available';
  };

  // Get the message count
  const getMessageCount = (session: SessionDetails): number => {
    return session.messages.length;
  };

  return (
    <div className="flex flex-col h-screen bg-bgApp">
      {/* Header */}
      <div className="flex items-center p-4 border-b border-borderSubtle">
        <button
          onClick={handleBackClick}
          className="flex items-center text-textPrimary hover:text-textSecondary transition-colors"
          aria-label={selectedSession ? 'Back to sessions list' : 'Back to chat'}
        >
          <ArrowLeft className="w-5 h-5 mr-2" />
          {selectedSession ? 'Back to sessions' : 'Back to chat'}
        </button>
        <h1 className="text-xl font-semibold text-textPrimary ml-4">
          {selectedSession ? 'Session Details' : 'Chat History'}
        </h1>

        {/* Refresh button - only show on sessions list */}
        {!selectedSession && !isLoading && (
          <button
            onClick={loadSessions}
            className="ml-auto flex items-center text-textSubtle hover:text-textPrimary transition-colors"
            aria-label="Refresh sessions"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              className={isLoading ? 'animate-spin' : ''}
            >
              <path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8" />
              <path d="M21 3v5h-5" />
            </svg>
            <span className="ml-2">Refresh</span>
          </button>
        )}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-4">
        {isLoading ? (
          <div className="flex justify-center items-center h-full">
            <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-textPrimary"></div>
          </div>
        ) : error && !selectedSession && sessions.length === 0 ? (
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
            <button
              onClick={loadSessions}
              className="px-4 py-2 bg-primary text-white rounded-md hover:bg-primary/90 transition-colors"
            >
              Try Again
            </button>
          </div>
        ) : selectedSession ? (
          // Session details view
          <div className="flex flex-col space-y-4">
            <div className="bg-bgSecondary p-4 rounded-lg border border-borderSubtle">
              <div className="flex justify-between items-start mb-4">
                <h2 className="text-lg font-medium text-textPrimary">
                  {selectedSession.description || selectedSession.session_id}
                </h2>
                <button
                  onClick={handleResumeSession}
                  className="px-4 py-2 bg-primary text-white rounded-md hover:bg-primary/90 transition-colors flex items-center"
                >
                  <MessageSquare className="w-4 h-4 mr-2" />
                  Resume Chat
                </button>
              </div>
              <div className="text-sm text-textSubtle mb-2">
                <span className="flex items-center">
                  <Clock className="w-4 h-4 mr-1" />
                  {new Date(selectedSession.messages[0]?.created * 1000).toLocaleString()}
                </span>
              </div>
              <div className="text-sm text-textSubtle">
                {getMessageCount(selectedSession)} messages in conversation
              </div>
            </div>

            <div className="space-y-4">
              {isLoadingSession ? (
                <div className="flex justify-center items-center py-12">
                  <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-textPrimary"></div>
                </div>
              ) : error ? (
                <div className="flex flex-col items-center justify-center py-8 text-textSubtle">
                  <div className="text-red-500 mb-4">
                    <svg
                      xmlns="http://www.w3.org/2000/svg"
                      width="32"
                      height="32"
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
                  <p className="text-md mb-2">Error Loading Session Details</p>
                  <p className="text-sm text-center mb-4">{error}</p>
                  <button
                    onClick={() => loadSessionDetails(selectedSession.session_id)}
                    className="px-4 py-2 bg-primary text-white rounded-md hover:bg-primary/90 transition-colors"
                  >
                    Try Again
                  </button>
                </div>
              ) : selectedSession?.messages?.length > 0 ? (
                selectedSession.messages.map((message, index) => (
                  <div
                    key={index}
                    className={`p-4 rounded-lg ${
                      message.role === 'user'
                        ? 'bg-bgSecondary border border-borderSubtle'
                        : 'bg-bgSubtle'
                    }`}
                  >
                    <div className="flex justify-between items-center mb-2">
                      <span className="font-medium text-textPrimary">
                        {message.role === 'user' ? 'You' : 'Goose'}
                      </span>
                      <span className="text-xs text-textSubtle">
                        {new Date(message.created * 1000).toLocaleTimeString()}
                      </span>
                    </div>
                    <div className="prose dark:prose-invert max-w-none">
                      {message.content.map((content, i) => (
                        <div key={i}>
                          {content.type === 'text' && (
                            <div className="whitespace-pre-wrap">{content.text}</div>
                          )}
                        </div>
                      ))}
                    </div>
                  </div>
                ))
              ) : (
                <div className="flex flex-col items-center justify-center py-8 text-textSubtle">
                  <MessageSquare className="w-12 h-12 mb-4" />
                  <p className="text-lg mb-2">No messages found</p>
                  <p className="text-sm">This session doesn't contain any messages</p>
                </div>
              )}
            </div>
          </div>
        ) : sessions.length > 0 ? (
          // Sessions list view
          <div className="grid gap-4">
            {sessions.map((session) => (
              <div
                key={session.id}
                onClick={() => handleSessionClick(session.id)}
                className="p-4 rounded-lg bg-bgSecondary border border-borderSubtle hover:border-borderPrimary cursor-pointer transition-all"
              >
                <div className="flex justify-between items-start">
                  <h3 className="text-lg font-medium text-textPrimary">
                    {session.description || session.id}
                  </h3>
                  <span className="text-sm text-textSubtle">{formatDate(session.modified)}</span>
                </div>
                <div className="flex items-center mt-2 text-textSubtle text-sm">
                  <ExternalLink className="w-4 h-4 mr-1" />
                  <span className="truncate max-w-[300px]">{session.path}</span>
                </div>
              </div>
            ))}
          </div>
        ) : (
          // Empty state
          <div className="flex flex-col items-center justify-center h-full text-textSubtle">
            <MessageSquare className="w-12 h-12 mb-4" />
            <p className="text-lg mb-2">No chat sessions found</p>
            <p className="text-sm">Your chat history will appear here</p>
          </div>
        )}
      </div>
    </div>
  );
};

export default SessionsView;
