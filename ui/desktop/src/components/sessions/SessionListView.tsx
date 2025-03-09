import React, { useEffect, useState } from 'react';
import { ViewConfig } from '../../App';
import { MessageSquare, Loader, AlertCircle, Calendar, ChevronRight, Folder } from 'lucide-react';
import { fetchSessions, type Session } from '../../sessions';
import { Card } from '../ui/card';
import { Button } from '../ui/button';
import BackButton from '../ui/BackButton';
import { ScrollArea } from '../ui/scroll-area';

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
  // eg. "10:39 PM, Feb 28, 2025"
  const formatDate = (dateString: string) => {
    try {
      const date = new Date(dateString);
      const time = new Intl.DateTimeFormat('en-US', {
        hour: 'numeric',
        minute: 'numeric',
        hour12: true,
      }).format(date);

      const dateStr = new Intl.DateTimeFormat('en-US', {
        month: 'short',
        day: 'numeric',
        year: 'numeric',
      }).format(date);

      return `${time}, ${dateStr}`;
    } catch (e) {
      return dateString;
    }
  };

  return (
    <div className="h-screen w-full">
      <div className="relative flex items-center h-[36px] w-full bg-bgSubtle"></div>

      <ScrollArea className="h-full w-full">
        <div className="flex flex-col pb-24">
          <div className="px-8 pt-6 pb-4">
            <BackButton onClick={() => setView('chat')} />
          </div>

          {/* Content Area */}
          <div className="flex flex-col mb-6 px-8">
            <h1 className="text-3xl font-medium text-textStandard">Previous goose sessions</h1>
            <h3 className="text-sm text-textSubtle mt-2">
              View previous goose sessions and their contents to pick up where you left off.
            </h3>
          </div>
          <div className="flex-1 overflow-y-auto p-4">
            {isLoading ? (
              <div className="flex justify-center items-center h-full">
                <Loader className="h-8 w-8 animate-spin text-textPrimary" />
              </div>
            ) : error ? (
              <div className="flex flex-col items-center justify-center h-full text-textSubtle">
                <AlertCircle className="h-12 w-12 text-red-500 mb-4" />
                <p className="text-lg mb-2">Error Loading Sessions</p>
                <p className="text-sm text-center mb-4">{error}</p>
                <Button onClick={loadSessions} variant="default">
                  Try Again
                </Button>
              </div>
            ) : sessions.length > 0 ? (
              <div className="grid gap-2">
                {sessions.map((session) => (
                  <Card
                    key={session.id}
                    onClick={() => onSelectSession(session.id)}
                    className="p-2 bg-bgSecondary hover:bg-bgSubtle cursor-pointer transition-all duration-150"
                  >
                    <div className="flex justify-between items-start">
                      <div className="w-full">
                        <h3 className="text-base font-medium text-textStandard truncate">
                          {session.metadata.description || session.id}
                        </h3>
                        <div className="flex gap-3">
                          <div className="flex items-center text-textSubtle text-sm">
                            <Calendar className="w-3 h-3 mr-1 flex-shrink-0" />
                            <span className="truncate">{formatDate(session.modified)}</span>
                          </div>
                          <div className="flex items-center text-textSubtle text-sm">
                            <Folder className="w-3 h-3 mr-1 flex-shrink-0" />
                            <span className="truncate">{session.metadata.working_dir}</span>
                          </div>
                        </div>
                      </div>

                      <div className="flex items-center gap-3">
                        <div className="flex flex-col items-end">
                          <div className="flex items-center text-sm text-textSubtle">
                            <span>{session.path.split('/').pop() || session.path}</span>
                          </div>
                          <div className="flex items-center mt-1 space-x-3 text-sm text-textSubtle">
                            <div className="flex items-center">
                              <MessageSquare className="w-3 h-3 mr-1" />
                              <span>{session.metadata.message_count}</span>
                            </div>
                            {session.metadata.total_tokens !== null && (
                              <div className="flex items-center">
                                <span>{session.metadata.total_tokens.toLocaleString()} tokens</span>
                              </div>
                            )}
                          </div>
                        </div>
                        <ChevronRight className="w-8 h-5 text-textSubtle" />
                      </div>
                    </div>
                  </Card>
                ))}
              </div>
            ) : (
              <div className="flex flex-col items-center justify-center h-full text-textSubtle">
                <MessageSquare className="h-12 w-12 mb-4" />
                <p className="text-lg mb-2">No chat sessions found</p>
                <p className="text-sm">Your chat history will appear here</p>
              </div>
            )}
          </div>
        </div>
      </ScrollArea>
    </div>
  );
};

export default SessionListView;
