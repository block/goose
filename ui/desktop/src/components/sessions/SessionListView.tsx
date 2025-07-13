import React, { useEffect, useState, useRef } from 'react';
import {
  MessageSquareText,
  Target,
  LoaderCircle,
  AlertCircle,
  Calendar,
  ChevronRight,
  Folder,
  Search,
} from 'lucide-react';
import { fetchSessions, searchSessionsContent, type Session } from '../../sessions';
import { Card } from '../ui/card';
import { Button } from '../ui/button';
import BackButton from '../ui/BackButton';
import { ScrollArea } from '../ui/scroll-area';
import { View, ViewOptions } from '../../App';
import { formatMessageTimestamp } from '../../utils/timeUtils';
import MoreMenuLayout from '../more_menu/MoreMenuLayout';
import { SearchView } from '../conversation/SearchView';
import { SearchHighlighter } from '../../utils/searchHighlighter';
import { wildcardMatch } from '../../utils/wildcardMatch';

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
  const [searchTerm, setSearchTerm] = useState<string>('');
  const [searchResults, setSearchResults] = useState<{
    count: number;
    currentIndex: number;
  } | null>(null);
  const [isSearchingContent, setIsSearchingContent] = useState(false);
  const [searchProgress, setSearchProgress] = useState({ current: 0, total: 0 });
  const [includeContent, setIncludeContent] = useState(true);
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
  const handleSearch = async (term: string, caseSensitive: boolean) => {
    if (!term) {
      // Reset content search match flags when clearing the search
      const resetSessions = sessions.map(session => ({
        ...session,
        contentSearchMatch: false
      }));
      setSessions(resetSessions);
      setFilteredSessions(resetSessions);
      setSearchResults(null);
      setIsSearchingContent(false);
      setSearchProgress({ current: 0, total: 0 });
      return;
    }

    const searchTerm = caseSensitive ? term : term.toLowerCase();
    const hasWildcard = term.includes('*');
    
    // First, filter by metadata (quick operation)
    let filtered = sessions.filter((session) => {
      const description = session.metadata.description || session.id;
      const path = session.path;
      const workingDir = session.metadata.working_dir;

      // If the search term contains wildcards, use wildcard matching
      if (hasWildcard) {
        return (
          wildcardMatch(description, searchTerm, caseSensitive) ||
          wildcardMatch(path, searchTerm, caseSensitive) ||
          wildcardMatch(workingDir, searchTerm, caseSensitive)
        );
      } else {
        // Otherwise use regular includes matching
        if (caseSensitive) {
          return (
            description.includes(searchTerm) ||
            path.includes(searchTerm) ||
            workingDir.includes(searchTerm)
          );
        } else {
          return (
            description.toLowerCase().includes(searchTerm.toLowerCase()) ||
            path.toLowerCase().includes(searchTerm.toLowerCase()) ||
            workingDir.toLowerCase().includes(searchTerm.toLowerCase())
          );
        }
      }
    });

    // Update filtered sessions with metadata matches first
    setFilteredSessions(filtered);
    setSearchResults(filtered.length > 0 ? { count: filtered.length, currentIndex: 1 } : null);

    // If content search is enabled, search session content
    if (includeContent) {
      setIsSearchingContent(true);
      
      try {
        // Search session content
        const sessionsWithContentSearch = await searchSessionsContent(
          sessions, 
          term, 
          caseSensitive,
          (current, total) => {
            setSearchProgress({ current, total });
          }
        );
        
        // Filter sessions that match either metadata or content
        filtered = sessionsWithContentSearch.filter(session => {
          // Check if session matches metadata criteria (reuse the logic above)
          const matchesMetadata = (() => {
            const description = session.metadata.description || session.id;
            const path = session.path;
            const workingDir = session.metadata.working_dir;

            if (hasWildcard) {
              return (
                wildcardMatch(description, searchTerm, caseSensitive) ||
                wildcardMatch(path, searchTerm, caseSensitive) ||
                wildcardMatch(workingDir, searchTerm, caseSensitive)
              );
            } else {
              if (caseSensitive) {
                return (
                  description.includes(searchTerm) ||
                  path.includes(searchTerm) ||
                  workingDir.includes(searchTerm)
                );
              } else {
                return (
                  description.toLowerCase().includes(searchTerm.toLowerCase()) ||
                  path.toLowerCase().includes(searchTerm.toLowerCase()) ||
                  workingDir.toLowerCase().includes(searchTerm.toLowerCase())
                );
              }
            }
          })();
          
          // Return true if either metadata or content matches
          return matchesMetadata || session.contentSearchMatch;
        });
        
        // Update filtered sessions with content matches included
        setFilteredSessions(filtered);
        setSearchResults(filtered.length > 0 ? { count: filtered.length, currentIndex: 1 } : null);
      } catch (err) {
        console.error('Error searching session content:', err);
      } finally {
        setIsSearchingContent(false);
      }
    }

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
      
      // Ensure all sessions have contentSearchMatch set to false initially
      const sessionsWithResetFlags = sessions.map(session => ({
        ...session,
        contentSearchMatch: false
      }));
      
      setSessions(sessionsWithResetFlags);
      setFilteredSessions(sessionsWithResetFlags);
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
              {session.contentSearchMatch && (
                <span className="ml-2 inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-blue-100 text-blue-800">
                  <Search className="w-3 h-3 mr-1" />
                  Content match
                </span>
              )}
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
        <div className="flex flex-col mb-4 px-8">
          <h1 className="text-3xl font-medium text-textStandard">Previous goose sessions</h1>
          <h3 className="text-sm text-textSubtle mt-2">
            View previous goose sessions and their contents to pick up where you left off.
          </h3>
        </div>

        {/* Permanent Search Input */}
        <div className="px-8 mb-4">
          <div className="relative">
            <div className="absolute inset-y-0 left-0 flex items-center pl-3 pointer-events-none">
              <svg className="w-4 h-4 text-textSubtle" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 20 20">
                <path stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="m19 19-4-4m0-7A7 7 0 1 1 1 8a7 7 0 0 1 14 0Z"/>
              </svg>
            </div>
            <input
              type="text" 
              className="bg-bgSecondary border border-borderSubtle text-textStandard text-sm rounded-lg block w-full pl-10 pr-10 p-2.5 focus:ring-1 focus:ring-borderProminent focus:border-borderProminent"
              placeholder="Search sessions (use * for wildcards)..."
              value={searchTerm}
              onChange={(e) => {
                const value = e.target.value;
                setSearchTerm(value);
                handleSearch(value, false);
              }}
              title="Search supports wildcards: use * to match any characters. Example: 'react*app' will match 'react app' and 'react native app'"
            />
            {searchTerm && (
              <button
                onClick={() => {
                  setSearchTerm('');
                  // Reset content search match flags when clearing the search
                  const resetSessions = sessions.map(session => ({
                    ...session,
                    contentSearchMatch: false
                  }));
                  setSessions(resetSessions);
                  setFilteredSessions(resetSessions);
                  setSearchResults(null);
                  setIsSearchingContent(false);
                  setSearchProgress({ current: 0, total: 0 });
                }}
                className="absolute inset-y-0 right-0 flex items-center pr-3"
                title="Clear search"
              >
                <svg className="w-4 h-4 text-textSubtle hover:text-textStandard" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 14 14">
                  <path stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="m1 1 6 6m0 0 6 6M7 7l6-6M7 7l-6 6"/>
                </svg>
              </button>
            )}
          </div>
          
          <div className="flex items-center justify-between mt-2">
            <div className="flex items-center">
              <input
                id="search-content"
                type="checkbox"
                checked={includeContent}
                onChange={(e) => {
                  const newIncludeContent = e.target.checked;
                  setIncludeContent(newIncludeContent);
                  
                  if (searchTerm) {
                    // If turning off content search, reset content match flags
                    if (!newIncludeContent) {
                      const resetSessions = sessions.map(session => ({
                        ...session,
                        contentSearchMatch: false
                      }));
                      setSessions(resetSessions);
                      
                      // Re-filter based on metadata only
                      const filtered = resetSessions.filter((session) => {
                        const description = session.metadata.description || session.id;
                        const path = session.path;
                        const workingDir = session.metadata.working_dir;
                        const hasWildcard = searchTerm.includes('*');
                        const term = searchTerm.toLowerCase();

                        if (hasWildcard) {
                          return (
                            wildcardMatch(description, term, false) ||
                            wildcardMatch(path, term, false) ||
                            wildcardMatch(workingDir, term, false)
                          );
                        } else {
                          return (
                            description.toLowerCase().includes(term) ||
                            path.toLowerCase().includes(term) ||
                            workingDir.toLowerCase().includes(term)
                          );
                        }
                      });
                      
                      setFilteredSessions(filtered);
                      setSearchResults(filtered.length > 0 ? { count: filtered.length, currentIndex: 1 } : null);
                    } else {
                      // If turning on content search, perform the search
                      handleSearch(searchTerm, false);
                    }
                  }
                }}
                className="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500"
              />
              <label htmlFor="search-content" className="ml-2 text-xs text-textSubtle">
                Search within session content
              </label>
            </div>
            
            <div className="text-xs text-textSubtle">
              <strong>Tip:</strong> Use * as a wildcard in search. Example: "react*app" matches "react app" and "react native app"
            </div>
          </div>
          
          {isSearchingContent && (
            <div className="mt-2 flex items-center text-xs text-textSubtle">
              <LoaderCircle className="h-3 w-3 animate-spin mr-1" />
              <span>
                Searching session content... ({searchProgress.current}/{searchProgress.total})
              </span>
            </div>
          )}
          
          {searchTerm && !isSearchingContent && (
            <div className="mt-2 text-xs">
              {filteredSessions.length > 0 ? (
                <span className="text-textSubtle">
                  Found <strong>{filteredSessions.length}</strong> matching session{filteredSessions.length !== 1 ? 's' : ''}
                </span>
              ) : sessions.length > 0 ? (
                <span className="text-amber-500">
                  No matching sessions found
                </span>
              ) : null}
            </div>
          )}
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
