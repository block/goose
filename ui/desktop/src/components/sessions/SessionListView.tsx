import React, { useEffect, useState, useMemo } from 'react';
import {
  MessageSquareText,
  Target,
  LoaderCircle,
  AlertCircle,
  Calendar,
  ChevronRight,
  Folder,
  ChevronLeft,
} from 'lucide-react';
import { fetchSessions, type Session } from '../../sessions';
import { Card } from '../ui/card';
import { Button } from '../ui/button';
import BackButton from '../ui/BackButton';
import { ScrollArea } from '../ui/scroll-area';
import { View, ViewOptions } from '../../App';
import { formatMessageTimestamp } from '../../utils/timeUtils';

interface SessionListViewProps {
  setView: (view: View, viewOptions?: ViewOptions) => void;
  onSelectSession: (sessionId: string) => void;
}

const PAGE_SIZE = 30;

const SessionListView: React.FC<SessionListViewProps> = ({ setView, onSelectSession }) => {
  const [allSessions, setAllSessions] = useState<Session[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [currentPage, setCurrentPage] = useState(1);

  useEffect(() => {
    loadSessions();
  }, []);

  const loadSessions = async () => {
    setIsLoading(true);
    setError(null);
    setCurrentPage(1);
    try {
      const response = await fetchSessions();
      setAllSessions(response.sessions);
    } catch (err) {
      console.error('Failed to load sessions:', err);
      setError('Failed to load sessions. Please try again later.');
      setAllSessions([]);
    } finally {
      setIsLoading(false);
    }
  };

  // Calculate pagination details using useMemo for efficiency
  const { paginatedSessions, totalPages } = useMemo(() => {
    const total = allSessions.length;
    const pages = Math.ceil(total / PAGE_SIZE);
    const startIndex = (currentPage - 1) * PAGE_SIZE;
    const endIndex = startIndex + PAGE_SIZE;
    const sessionsSlice = allSessions.slice(startIndex, endIndex);
    return { paginatedSessions: sessionsSlice, totalPages: pages };
  }, [allSessions, currentPage]);

  const handlePreviousPage = () => {
    setCurrentPage((prev) => Math.max(prev - 1, 1));
  };

  const handleNextPage = () => {
    setCurrentPage((prev) => Math.min(prev + 1, totalPages));
  };

  return (
    <div className="h-screen w-full flex flex-col">
      <div className="relative flex items-center h-[36px] w-full bg-bgSubtle flex-shrink-0"></div>
      <div className="px-8 pt-6 pb-4 flex-shrink-0">
        <BackButton onClick={() => setView('chat')} />
      </div>
      <div className="flex flex-col mb-6 px-8 flex-shrink-0">
        <h1 className="text-3xl font-medium text-textStandard">Previous goose sessions</h1>
        <h3 className="text-sm text-textSubtle mt-2">
          View previous goose sessions and their contents to pick up where you left off.
        </h3>
      </div>
      <ScrollArea className="flex-grow w-full">
        <div className="flex flex-col p-4">
          {isLoading ? (
            <div className="flex justify-center items-center h-full">
              <LoaderCircle className="h-8 w-8 animate-spin text-textPrimary" />
            </div>
          ) : error ? (
            <div className="flex flex-col items-center justify-center h-full text-textSubtle mt-10">
              <AlertCircle className="h-12 w-12 text-red-500 mb-4" />
              <p className="text-lg mb-2">Error Loading Sessions</p>
              <p className="text-sm text-center mb-4">{error}</p>
              <Button onClick={loadSessions} variant="default">
                Try Again
              </Button>
            </div>
          ) : allSessions.length > 0 ? (
            <div className="grid gap-2">
              {paginatedSessions.map((session) => (
                <Card
                  key={session.id}
                  onClick={() => onSelectSession(session.id)}
                  className="p-2 bg-bgSecondary hover:bg-bgSubtle cursor-pointer transition-all duration-150"
                >
                  <div className="flex justify-between items-start gap-4">
                    <div className="min-w-0 flex-1">
                      <h3 className="text-base font-medium text-textStandard truncate max-w-[50vw]">
                        {session.metadata.description || session.id}
                      </h3>
                      <div className="flex gap-3 min-w-0">
                        <div className="flex items-center text-textSubtle text-sm shrink-0">
                          <Calendar className="w-3 h-3 mr-1 flex-shrink-0" />
                          <span>{formatMessageTimestamp(Date.parse(session.modified) / 1000)}</span>
                        </div>
                        <div className="flex items-center text-textSubtle text-sm min-w-0">
                          <Folder className="w-3 h-3 mr-1 flex-shrink-0" />
                          <span className="truncate">{session.metadata.working_dir}</span>
                        </div>
                      </div>
                    </div>

                    <div className="flex items-center gap-3 shrink-0">
                      <div className="flex flex-col items-end">
                        <div className="flex items-center text-sm text-textSubtle">
                          {/* Displaying full path might be too long, maybe just filename? */}
                          {/* <span className="truncate">{session.path.split('/').pop() || session.path}</span> */}
                        </div>
                        <div className="flex items-center mt-1 space-x-2 sm:space-x-3 text-sm text-textSubtle">
                          <div
                            className="flex items-center"
                            title={`${session.metadata.message_count} messages`}
                          >
                            <MessageSquareText className="w-3 h-3 mr-1" />
                            <span>{session.metadata.message_count}</span>
                          </div>
                          {session.metadata.total_tokens !== null && (
                            <div
                              className="flex items-center"
                              title={`${session.metadata.total_tokens.toLocaleString()} tokens`}
                            >
                              <Target className="w-3 h-3 mr-1" />
                              {/* Maybe use compact notation for large numbers? */}
                              <span>{session.metadata.total_tokens.toLocaleString()}</span>
                            </div>
                          )}
                        </div>
                      </div>
                      <ChevronRight className="w-5 h-5 sm:w-8 sm:h-5 text-textSubtle" />
                    </div>
                  </div>
                </Card>
              ))}
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center h-full text-textSubtle mt-10">
              <MessageSquareText className="h-12 w-12 mb-4" />
              <p className="text-lg mb-2">No chat sessions found</p>
              <p className="text-sm">Your chat history will appear here</p>
            </div>
          )}
        </div>
      </ScrollArea>
      {!isLoading && !error && totalPages > 1 && (
        <div className="flex justify-center items-center p-4 border-t border-borderSubtle flex-shrink-0">
          <Button
            variant="ghost"
            size="sm"
            onClick={handlePreviousPage}
            disabled={currentPage === 1}
            className="mr-2 dark:text-textStandard"
          >
            <ChevronLeft className="w-4 h-4 mr-1" />
            Previous
          </Button>
          <span className="text-sm text-textSubtle mx-2">
            Page {currentPage} of {totalPages}
          </span>
          <Button
            variant="ghost"
            size="sm"
            onClick={handleNextPage}
            disabled={currentPage === totalPages}
            className="ml-2 dark:text-textStandard"
          >
            Next
            <ChevronRight className="w-4 h-4 ml-1" />
          </Button>
        </div>
      )}
    </div>
  );
};

export default SessionListView;
