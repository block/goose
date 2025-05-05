import React, { useEffect, useState, useRef } from 'react';
import {
  MessageSquareText,
  Target,
  LoaderCircle,
  AlertCircle,
  Calendar,
  ChevronRight,
  Folder,
} from 'lucide-react';
import { fetchSessions, type Session } from '../../sessions';
import { Card } from '../ui/card';
import { Button } from '../ui/button';
import BackButton from '../ui/BackButton';
import { ScrollArea } from '../ui/scroll-area';
import { View, ViewOptions } from '../../App';
import { formatMessageTimestamp } from '../../utils/timeUtils';
import MoreMenuLayout from '../more_menu/MoreMenuLayout';
import { SearchView } from '../conversation/SearchView';
import { SearchHighlighter } from '../../utils/searchHighlighter';

interface SearchContainerElement extends HTMLDivElement {
  _searchHighlighter: SearchHighlighter | null;
}

interface SessionListViewProps {
  setView: (view: View, viewOptions?: ViewOptions) => void;
  onSelectSession: (sessionId: string) => void;
}

const ITEM_HEIGHT = 90; // Adjust based on your card height
const BUFFER_SIZE = 5; // Number of items to render above/below viewport

const SessionListView: React.FC<SessionListViewProps> = ({ setView, onSelectSession }) => {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [filteredSessions, setFilteredSessions] = useState<Session[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchResults, setSearchResults] = useState<{
    count: number;
    currentIndex: number;
  } | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [visibleRange, setVisibleRange] = useState({ start: 0, end: 20 });

  useEffect(() => {
    loadSessions();
  }, []);

  // Handle scroll events to update visible range
  useEffect(() => {
    const viewportEl = containerRef.current?.closest('[data-radix-scroll-area-viewport]');
    if (!viewportEl) return;

    const handleScroll = () => {
      const scrollTop = viewportEl.scrollTop;
      const viewportHeight = viewportEl.clientHeight;

      const start = Math.max(0, Math.floor(scrollTop / ITEM_HEIGHT) - BUFFER_SIZE);
      const end = Math.min(
        filteredSessions.length,
        Math.ceil((scrollTop + viewportHeight) / ITEM_HEIGHT) + BUFFER_SIZE
      );

      setVisibleRange({ start, end });
    };

    handleScroll(); // Initial calculation
    viewportEl.addEventListener('scroll', handleScroll);

    const resizeObserver = new ResizeObserver(handleScroll);
    resizeObserver.observe(viewportEl);

    return () => {
      viewportEl.removeEventListener('scroll', handleScroll);
      resizeObserver.disconnect();
    };
  }, [filteredSessions.length]);

  // Filter sessions when search term or case sensitivity changes
  const handleSearch = (term: string, caseSensitive: boolean) => {
    if (!term) {
      setFilteredSessions(sessions);
      setSearchResults(null);
      return;
    }

    const searchTerm = caseSensitive ? term : term.toLowerCase();
    const filtered = sessions.filter((session) => {
      const description = session.metadata.description || session.id;
      const path = session.path;
      const workingDir = session.metadata.working_dir;

      if (caseSensitive) {
        return (
          description.includes(searchTerm) ||
          path.includes(searchTerm) ||
          workingDir.includes(searchTerm)
        );
      } else {
        return (
          description.toLowerCase().includes(searchTerm) ||
          path.toLowerCase().includes(searchTerm) ||
          workingDir.toLowerCase().includes(searchTerm)
        );
      }
    });

    setFilteredSessions(filtered);
    setSearchResults(filtered.length > 0 ? { count: filtered.length, currentIndex: 1 } : null);

    // Reset scroll position when search changes
    const viewportEl = containerRef.current?.closest('[data-radix-scroll-area-viewport]');
    if (viewportEl) {
      viewportEl.scrollTop = 0;
    }
    setVisibleRange({ start: 0, end: 20 });
  };

  const loadSessions = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const sessions = await fetchSessions();
      setSessions(sessions);
      setFilteredSessions(sessions);
    } catch (err) {
      console.error('Failed to load sessions:', err);
      setError('Failed to load sessions. Please try again later.');
      setSessions([]);
      setFilteredSessions([]);
    } finally {
      setIsLoading(false);
    }
  };

  // Handle search result navigation
  const handleSearchNavigation = (direction: 'next' | 'prev') => {
    if (!searchResults || filteredSessions.length === 0) return;

    let newIndex: number;
    if (direction === 'next') {
      newIndex = (searchResults.currentIndex % filteredSessions.length) + 1;
    } else {
      newIndex =
        searchResults.currentIndex === 1 ? filteredSessions.length : searchResults.currentIndex - 1;
    }

    setSearchResults({ ...searchResults, currentIndex: newIndex });

    // Find the SearchView's container element
    const searchContainer =
      containerRef.current?.querySelector<SearchContainerElement>('.search-container');
    if (searchContainer?._searchHighlighter) {
      // Update the current match in the highlighter
      searchContainer._searchHighlighter.setCurrentMatch(newIndex - 1, true);
    }
  };

  // Render a session item
  const SessionItem = React.memo(function SessionItem({ session }: { session: Session }) {
    return (
      <Card
        onClick={() => onSelectSession(session.id)}
        className="p-2 mx-4 mb-2 bg-bgSecondary hover:bg-bgSubtle cursor-pointer transition-all duration-150"
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
                <span>{session.path.split('/').pop() || session.path}</span>
              </div>
              <div className="flex items-center mt-1 space-x-3 text-sm text-textSubtle">
                <div className="flex items-center">
                  <MessageSquareText className="w-3 h-3 mr-1" />
                  <span>{session.metadata.message_count}</span>
                </div>
                {session.metadata.total_tokens !== null && (
                  <div className="flex items-center">
                    <Target className="w-3 h-3 mr-1" />
                    <span>{session.metadata.total_tokens.toLocaleString()}</span>
                  </div>
                )}
              </div>
            </div>
            <ChevronRight className="w-8 h-5 text-textSubtle" />
          </div>
        </div>
      </Card>
    );
  });

  const renderContent = () => {
    if (isLoading) {
      return (
        <div className="flex justify-center items-center h-full">
          <LoaderCircle className="h-8 w-8 animate-spin text-textPrimary" />
        </div>
      );
    }

    if (error) {
      return (
        <div className="flex flex-col items-center justify-center h-full text-textSubtle">
          <AlertCircle className="h-12 w-12 text-red-500 mb-4" />
          <p className="text-lg mb-2">Error Loading Sessions</p>
          <p className="text-sm text-center mb-4">{error}</p>
          <Button onClick={loadSessions} variant="default">
            Try Again
          </Button>
        </div>
      );
    }

    if (filteredSessions.length === 0) {
      if (searchResults === null && sessions.length > 0) {
        return (
          <div className="flex flex-col items-center justify-center h-full text-textSubtle mt-4">
            <MessageSquareText className="h-12 w-12 mb-4" />
            <p className="text-lg mb-2">No matching sessions found</p>
            <p className="text-sm">Try adjusting your search terms</p>
          </div>
        );
      }
      return (
        <div className="flex flex-col items-center justify-center h-full text-textSubtle">
          <MessageSquareText className="h-12 w-12 mb-4" />
          <p className="text-lg mb-2">No chat sessions found</p>
          <p className="text-sm">Your chat history will appear here</p>
        </div>
      );
    }

    const visibleSessions = filteredSessions.slice(visibleRange.start, visibleRange.end);

    return (
      <div style={{ height: filteredSessions.length * ITEM_HEIGHT }} className="relative">
        <div
          style={{
            position: 'absolute',
            top: visibleRange.start * ITEM_HEIGHT,
            width: '100%',
          }}
        >
          {visibleSessions.map((session) => (
            <SessionItem key={session.id} session={session} />
          ))}
        </div>
      </div>
    );
  };

  return (
    <div className="h-screen w-full flex flex-col">
      <MoreMenuLayout showMenu={false} />

      <div className="flex-1 flex flex-col min-h-0">
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

        <div className="flex-1 min-h-0 relative">
          <ScrollArea className="h-full" data-search-scroll-area>
            <div ref={containerRef} className="h-full relative">
              <SearchView
                onSearch={handleSearch}
                onNavigate={handleSearchNavigation}
                searchResults={searchResults}
                className="relative"
              >
                {renderContent()}
              </SearchView>
            </div>
          </ScrollArea>
        </div>
      </div>
    </div>
  );
};

export default SessionListView;
