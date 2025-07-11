import React, { useEffect, useState, useRef, useMemo } from 'react';
import { MessageSquareText, Target, AlertCircle, Calendar, Folder } from 'lucide-react';
import { VariableSizeList as List } from 'react-window';
import { fetchSessions, type Session } from '../../sessions';
import { Card } from '../ui/card';
import { Button } from '../ui/button';
import { ScrollArea } from '../ui/scroll-area';
import { View, ViewOptions } from '../../App';
import { formatMessageTimestamp } from '../../utils/timeUtils';
import { SearchView } from '../conversation/SearchView';
import { SearchHighlighter } from '../../utils/searchHighlighter';
import { MainPanelLayout } from '../Layout/MainPanelLayout';
import { groupSessionsByDate, type DateGroup } from '../../utils/dateUtils';
import { Skeleton } from '../ui/skeleton';

interface SearchContainerElement extends HTMLDivElement {
  _searchHighlighter: SearchHighlighter | null;
}

interface SessionListViewProps {
  setView: (view: View, viewOptions?: ViewOptions) => void;
  onSelectSession: (sessionId: string) => void;
}

// Flattened item structure for virtualization
interface FlattenedItem {
  type: 'header' | 'session';
  data: DateGroup | Session;
  index: number;
}

const SessionListView: React.FC<SessionListViewProps> = ({ onSelectSession }) => {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [filteredSessions, setFilteredSessions] = useState<Session[]>([]);
  const [dateGroups, setDateGroups] = useState<DateGroup[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [showSkeleton, setShowSkeleton] = useState(true);
  const [showContent, setShowContent] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searchResults, setSearchResults] = useState<{
    count: number;
    currentIndex: number;
  } | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<List>(null);

  // Flatten date groups into a single array for virtualization
  const flattenedItems = useMemo((): FlattenedItem[] => {
    const items: FlattenedItem[] = [];
    let index = 0;

    dateGroups.forEach((group) => {
      // Add header
      items.push({
        type: 'header',
        data: group,
        index: index++,
      });

      // Add sessions
      group.sessions.forEach((session) => {
        items.push({
          type: 'session',
          data: session,
          index: index++,
        });
      });
    });

    return items;
  }, [dateGroups]);

  useEffect(() => {
    loadSessions();
  }, []);

  // Minimum loading time to prevent skeleton flash
  useEffect(() => {
    if (!isLoading && showSkeleton) {
      const timer = setTimeout(() => {
        setShowSkeleton(false);
        // Add a small delay before showing content for fade-in effect
        setTimeout(() => {
          setShowContent(true);
        }, 50);
      }, 300); // Show skeleton for at least 300ms

      // eslint-disable-next-line no-undef
      return () => clearTimeout(timer);
    }
    return () => void 0;
  }, [isLoading, showSkeleton]);

  // Update date groups when filtered sessions change
  useEffect(() => {
    if (filteredSessions.length > 0) {
      const groups = groupSessionsByDate(filteredSessions);
      setDateGroups(groups);
    } else {
      setDateGroups([]);
    }
  }, [filteredSessions]);

  // Reset list cache when items change
  useEffect(() => {
    if (listRef.current) {
      listRef.current.resetAfterIndex(0);
    }
  }, [flattenedItems]);

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
  };

  const loadSessions = async () => {
    setIsLoading(true);
    setShowSkeleton(true);
    setShowContent(false);
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
        className="h-full py-3 px-4 hover:shadow-default cursor-pointer transition-all duration-150 flex flex-col justify-between"
      >
        <div className="flex-1">
          <h3 className="text-base truncate mb-1">{session.metadata.description || session.id}</h3>
          <div className="flex items-center text-text-muted text-xs mb-1">
            <Calendar className="w-3 h-3 mr-1 flex-shrink-0" />
            <span>{formatMessageTimestamp(Date.parse(session.modified) / 1000)}</span>
          </div>
          <div className="flex items-center text-text-muted text-xs mb-1">
            <Folder className="w-3 h-3 mr-1 flex-shrink-0" />
            <span className="truncate">{session.metadata.working_dir}</span>
          </div>
        </div>

        <div className="flex items-center justify-between mt-1 pt-2">
          <div className="flex items-center space-x-3 text-xs text-text-muted">
            <div className="flex items-center">
              <MessageSquareText className="w-3 h-3 mr-1" />
              <span className="font-mono">{session.metadata.message_count}</span>
            </div>
            {session.metadata.total_tokens !== null && (
              <div className="flex items-center">
                <Target className="w-3 h-3 mr-1" />
                <span className="font-mono">{session.metadata.total_tokens.toLocaleString()}</span>
              </div>
            )}
          </div>
        </div>
      </Card>
    );
  });

  // Render skeleton loader for session items
  const SessionSkeleton = () => (
    <Card className="h-full p-3 bg-background-default flex flex-col justify-between">
      <div className="flex-1">
        <Skeleton className="h-5 w-3/4 mb-2" />
        <Skeleton className="h-4 w-24 mb-2" />
        <Skeleton className="h-4 w-32 mb-2" />
        <Skeleton className="h-4 w-20" />
      </div>
      <div className="flex items-center justify-between mt-3 pt-2 border-t border-border-subtle">
        <div className="flex items-center space-x-3">
          <Skeleton className="h-4 w-8" />
          <Skeleton className="h-4 w-12" />
        </div>
      </div>
    </Card>
  );

  const renderContent = () => {
    if (isLoading || showSkeleton) {
      return (
        <div className="space-y-6">
          <div className="space-y-3">
            <Skeleton className="h-6 w-24" />
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5 gap-4">
              <SessionSkeleton />
              <SessionSkeleton />
              <SessionSkeleton />
              <SessionSkeleton />
            </div>
          </div>
        </div>
      );
    }

    if (error) {
      return (
        <div className="flex flex-col items-center justify-center h-full text-text-muted">
          <AlertCircle className="h-12 w-12 text-red-500 mb-4" />
          <p className="text-lg mb-2">Error Loading Sessions</p>
          <p className="text-sm text-center mb-4">{error}</p>
          <Button onClick={loadSessions} variant="default">
            Try Again
          </Button>
        </div>
      );
    }

    if (sessions.length === 0) {
      return (
        <div className="flex flex-col justify-center h-full text-text-muted">
          <MessageSquareText className="h-12 w-12 mb-4" />
          <p className="text-lg mb-2">No chat sessions found</p>
          <p className="text-sm">Your chat history will appear here</p>
        </div>
      );
    }

    if (dateGroups.length === 0 && searchResults !== null) {
      return (
        <div className="flex flex-col items-center justify-center h-full text-text-muted mt-4">
          <MessageSquareText className="h-12 w-12 mb-4" />
          <p className="text-lg mb-2">No matching sessions found</p>
          <p className="text-sm">Try adjusting your search terms</p>
        </div>
      );
    }

    // For regular rendering in grid layout
    return (
      <div className="space-y-8">
        {dateGroups.map((group) => (
          <div key={group.label} className="space-y-4">
            <div className="sticky top-0 z-10 bg-background-default/95 backdrop-blur-sm">
              <h2 className="text-text-muted">{group.label}</h2>
            </div>
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5 gap-4">
              {group.sessions.map((session) => (
                <SessionItem key={session.id} session={session} />
              ))}
            </div>
          </div>
        ))}
      </div>
    );
  };

  return (
    <MainPanelLayout>
      <div className="flex-1 flex flex-col min-h-0">
        <div className="bg-background-default px-8 pb-8 pt-16">
          <div className="flex flex-col animate-in fade-in duration-500">
            <div className="flex justify-between items-center mb-1">
              <h1 className="text-4xl font-light">Chat history</h1>
            </div>
            <p className="text-sm text-text-muted mb-4">
              View and search your past conversations with Goose.
            </p>
          </div>
        </div>

        <div className="flex-1 min-h-0 relative px-8">
          <ScrollArea className="h-full">
            <div
              className={`h-full relative transition-all duration-300 ${
                showContent ? 'opacity-100 animate-in fade-in' : 'opacity-0'
              }`}
            >
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
    </MainPanelLayout>
  );
};

export default SessionListView;
