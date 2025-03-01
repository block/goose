import React, { useEffect, useState } from 'react';
import { ViewConfig } from '../App';

// Define the type for a chat session
interface Session {
  id: string;
  title: string;
  date: string;
  preview: string;
}

interface SessionsViewProps {
  setView: (view: ViewConfig['view'], viewOptions?: Record<any, any>) => void;
}

const SessionsView: React.FC<SessionsViewProps> = ({ setView }) => {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    // Simulate loading sessions from storage
    // In a real implementation, you would fetch this from your storage mechanism
    const loadSessions = async () => {
      setIsLoading(true);
      try {
        // Mock data for now - replace with actual data fetching
        const mockSessions: Session[] = [
          {
            id: '1',
            title: 'React Component Discussion',
            date: '2025-02-28',
            preview: 'How to structure React components for better reusability...',
          },
          {
            id: '2',
            title: 'Electron IPC Communication',
            date: '2025-02-27',
            preview: 'Setting up communication between main and renderer processes...',
          },
          {
            id: '3',
            title: 'TypeScript Type Definitions',
            date: '2025-02-26',
            preview: 'Creating proper type definitions for complex objects...',
          },
        ];

        // Simulate network delay
        setTimeout(() => {
          setSessions(mockSessions);
          setIsLoading(false);
        }, 500);
      } catch (error) {
        console.error('Failed to load sessions:', error);
        setIsLoading(false);
      }
    };

    loadSessions();
  }, []);

  const handleBackClick = () => {
    setView('chat');
  };

  const handleSessionClick = (sessionId: string) => {
    // In a real implementation, you would navigate to the selected session
    console.log(`Selected session: ${sessionId}`);
    // Example: setView('chat', { sessionId });
  };

  return (
    <div className="flex flex-col h-screen bg-bgApp">
      {/* Header */}
      <div className="flex items-center p-4 border-b border-borderPrimary">
        <button
          onClick={handleBackClick}
          className="flex items-center text-textPrimary hover:text-textSecondary transition-colors"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="24"
            height="24"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            className="mr-2"
          >
            <path d="M19 12H5M12 19l-7-7 7-7" />
          </svg>
          Back
        </button>
        <h1 className="text-xl font-semibold text-textPrimary ml-4">Session History</h1>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-4">
        {isLoading ? (
          <div className="flex justify-center items-center h-full">
            <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-textPrimary"></div>
          </div>
        ) : sessions.length > 0 ? (
          <div className="grid gap-4">
            {sessions.map((session) => (
              <div
                key={session.id}
                onClick={() => handleSessionClick(session.id)}
                className="p-4 rounded-lg bg-bgSecondary border border-borderPrimary hover:border-borderSecondary cursor-pointer transition-all"
              >
                <div className="flex justify-between items-start">
                  <h3 className="text-lg font-medium text-textPrimary">{session.title}</h3>
                  <span className="text-sm text-textSecondary">{session.date}</span>
                </div>
                <p className="mt-2 text-textSecondary line-clamp-2">{session.preview}</p>
              </div>
            ))}
          </div>
        ) : (
          <div className="flex flex-col items-center justify-center h-full text-textSecondary">
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
            <p>No chat sessions found</p>
          </div>
        )}
      </div>
    </div>
  );
};

export default SessionsView;
