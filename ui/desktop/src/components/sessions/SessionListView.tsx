import React, { useEffect, useState, useRef, useCallback } from 'react';
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

// Virtual scrolling constants
const ITEM_HEIGHT = 68; // Fixed height for all session items
const BUFFER_SIZE = 5; // Number of items to render above/below viewport
const OVERSCAN = 3; // Additional items to render for smoother scrolling

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
  const scrollElementRef = useRef<HTMLDivElement | null>(null);
  const [visibleRange, setVisibleRange] = useState({ start: 0, end: 20 });
  const shouldScrollToTop = useRef(false);

  useEffect(() => {
    loadSessions();
  }, []);

  // Calculate item positions with fixed height
  const calculateItemPositions = (sessions: Session[]) => {
    const totalHeight = sessions.length * ITEM_HEIGHT;
    const positions: { top: number; height: number }[] = [];

    for (let i = 0; i < sessions.length; i++) {
      positions.push({
        top: i * ITEM_HEIGHT,
        height: ITEM_HEIGHT,
      });
    }

    return { positions, totalHeight };
  };

  // Update visible range based on scroll position with fixed heights
  const updateVisibleRange = useCallback(
    (scrollTop: number, viewportHeight: number) => {
      if (filteredSessions.length === 0) return;

      // Simple calculation with fixed heights
      const start = Math.max(0, Math.floor(scrollTop / ITEM_HEIGHT) - OVERSCAN - BUFFER_SIZE);
      const visibleCount = Math.ceil(viewportHeight / ITEM_HEIGHT);
      const end = Math.min(
        filteredSessions.length,
        start + visibleCount + OVERSCAN + BUFFER_SIZE * 2
      );

      setVisibleRange({ start, end });
    },
    [filteredSessions.length]
  );

  // Handle scroll events
  useEffect(() => {
    // Find the Radix ScrollArea viewport
    const scrollElement = containerRef.current?.closest(
      '[data-radix-scroll-area-viewport]'
    ) as HTMLDivElement;
    if (!scrollElement) return;

    // Store reference for other functions
    scrollElementRef.current = scrollElement;

    const handleScroll = () => {
      const scrollTop = scrollElement.scrollTop;
      const viewportHeight = scrollElement.clientHeight;

      updateVisibleRange(scrollTop, viewportHeight);
    };

    // Initial calculation
    handleScroll();

    // Add scroll listener with throttling
    let ticking = false;
    const throttledScroll = () => {
      if (!ticking) {
        requestAnimationFrame(() => {
          handleScroll();
          ticking = false;
        });
        ticking = true;
      }
    };

    scrollElement.addEventListener('scroll', throttledScroll, { passive: true });

    // Handle resize
    const resizeObserver = new ResizeObserver(() => {
      handleScroll();
    });
    resizeObserver.observe(scrollElement);

    return () => {
      scrollElement.removeEventListener('scroll', throttledScroll);
      resizeObserver.disconnect();
    };
  }, [filteredSessions.length, updateVisibleRange]);

  // Update visible range when filtered sessions change
  useEffect(() => {
    const scrollElement = scrollElementRef.current;
    if (scrollElement) {
      // If we should scroll to top (after clearing search), do it now
      if (shouldScrollToTop.current) {
        scrollElement.scrollTop = 0;
        setVisibleRange({ start: 0, end: 20 });
        shouldScrollToTop.current = false;
      }
      updateVisibleRange(scrollElement.scrollTop, scrollElement.clientHeight);
    }
  }, [filteredSessions, updateVisibleRange]);

  // Filter sessions when search term or case sensitivity changes
  const handleSearch = (term: string, caseSensitive: boolean) => {
    if (!term) {
      // Mark that we should scroll to top after the state updates
      shouldScrollToTop.current = true;
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
    setTimeout(() => {
      if (scrollElementRef.current) {
        scrollElementRef.current.scrollTop = 0;
      }
      setVisibleRange({ start: 0, end: 20 });
    }, 0);
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

  // Render a session item (simplified without height measurement)
  const SessionItem = React.memo(function SessionItem({
    session,
    style,
  }: {
    session: Session;
    style?: React.CSSProperties;
  }) {
    return (
      <div style={style}>
        <Card
          onClick={() => onSelectSession(session.id)}
          className="p-2 mx-4 mb-2 bg-bgSecondary hover:bg-bgSubtle cursor-pointer transition-all duration-150 will-change-transform"
          style={{ transform: 'translateZ(0)' }}
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
      </div>
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

    // Calculate positions and total height for virtual scrolling
    const { positions, totalHeight } = calculateItemPositions(filteredSessions);
    const visibleSessions = filteredSessions.slice(visibleRange.start, visibleRange.end);

    return (
      <div style={{ height: totalHeight, position: 'relative' }}>
        {visibleSessions.map((session, index) => {
          const actualIndex = visibleRange.start + index;
          const position = positions[actualIndex];

          return (
            <SessionItem
              key={session.id}
              session={session}
              style={{
                position: 'absolute',
                top: position.top,
                left: 0,
                right: 0,
              }}
            />
          );
        })}
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
            View previous goose sessions and their contents to pick up where you left off. ⌘F to
            search
          </h3>
        </div>

        <div className="flex-1 min-h-0 relative">
          <ScrollArea className="h-full" data-search-scroll-area>
            <div
              ref={containerRef}
              className="h-full relative"
              style={{ transform: 'translateZ(0)' }}
            >
              <SearchView
                onSearch={handleSearch}
                onNavigate={handleSearchNavigation}
                searchResults={searchResults}
                placeholder="Search session history..."
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
