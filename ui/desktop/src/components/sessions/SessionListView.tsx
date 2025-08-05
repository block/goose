import React, { useEffect, useState, useRef, useCallback, useMemo, startTransition } from 'react';
import { MessageSquareText, Target, AlertCircle, Calendar, Folder, Edit2 } from 'lucide-react';
import { fetchSessions, updateSessionMetadata, type Session } from '../../sessions';
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
import { toast } from 'react-toastify';

interface EditSessionModalProps {
  session: Session | null;
  isOpen: boolean;
  onClose: () => void;
  onSave: (sessionId: string, newDescription: string) => Promise<void>;
  disabled?: boolean;
}

const EditSessionModal = React.memo<EditSessionModalProps>(
  ({ session, isOpen, onClose, onSave, disabled = false }) => {
    const [description, setDescription] = useState('');
    const [isUpdating, setIsUpdating] = useState(false);

    useEffect(() => {
      if (session && isOpen) {
        setDescription(session.metadata.description || session.id);
      } else if (!isOpen) {
        // Reset state when modal closes
        setDescription('');
        setIsUpdating(false);
      }
    }, [session, isOpen]);

    const handleSave = useCallback(async () => {
      if (!session || disabled) return;

      const trimmedDescription = description.trim();
      if (trimmedDescription === session.metadata.description) {
        onClose();
        return;
      }

      setIsUpdating(true);
      try {
        await updateSessionMetadata(session.id, trimmedDescription);
        await onSave(session.id, trimmedDescription);

        // Close modal, then show success toast on a timeout to let the UI update complete.
        onClose();
        setTimeout(() => {
          toast.success('Session description updated successfully');
        }, 300);
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Unknown error occurred';
        console.error('Failed to update session description:', errorMessage);
        toast.error(`Failed to update session description: ${errorMessage}`);
        // Reset to original description on error
        setDescription(session.metadata.description || session.id);
      } finally {
        setIsUpdating(false);
      }
    }, [session, description, onSave, onClose, disabled]);

    const handleCancel = useCallback(() => {
      if (!isUpdating) {
        onClose();
      }
    }, [onClose, isUpdating]);

    const handleKeyDown = useCallback(
      (e: React.KeyboardEvent<HTMLInputElement>) => {
        if (e.key === 'Enter' && !isUpdating) {
          handleSave();
        } else if (e.key === 'Escape' && !isUpdating) {
          handleCancel();
        }
      },
      [handleSave, handleCancel, isUpdating]
    );

    const handleInputChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
      setDescription(e.target.value);
    }, []);

    if (!isOpen || !session) return null;

    return (
      <div className="fixed inset-0 z-[300] flex items-center justify-center bg-black/50">
        <div className="bg-background-default border border-border-subtle rounded-lg p-6 w-[500px] max-w-[90vw]">
          <h3 className="text-lg font-medium text-text-standard mb-4">Edit Session Description</h3>

          <div className="space-y-4">
            <div>
              <input
                id="session-description"
                type="text"
                value={description}
                onChange={handleInputChange}
                className="w-full p-3 border border-border-subtle rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500"
                placeholder="Enter session description"
                autoFocus
                maxLength={200}
                onKeyDown={handleKeyDown}
                disabled={isUpdating || disabled}
              />
            </div>
          </div>

          <div className="flex justify-end space-x-3 mt-6">
            <Button onClick={handleCancel} variant="ghost" disabled={isUpdating || disabled}>
              Cancel
            </Button>
            <Button
              onClick={handleSave}
              disabled={!description.trim() || isUpdating || disabled}
              variant="default"
            >
              {isUpdating ? 'Saving...' : 'Save'}
            </Button>
          </div>
        </div>
      </div>
    );
  }
);

EditSessionModal.displayName = 'EditSessionModal';

// Debounce hook for search
function useDebounce<T>(value: T, delay: number): T {
  const [debouncedValue, setDebouncedValue] = useState<T>(value);

  useEffect(() => {
    const handler = setTimeout(() => {
      setDebouncedValue(value);
    }, delay);

    return () => {
      window.clearTimeout(handler);
    };
  }, [value, delay]);

  return debouncedValue;
}

interface SearchContainerElement extends HTMLDivElement {
  _searchHighlighter: SearchHighlighter | null;
}

interface SessionListViewProps {
  setView: (view: View, viewOptions?: ViewOptions) => void;
  onSelectSession: (sessionId: string) => void;
}

const SessionListView: React.FC<SessionListViewProps> = React.memo(({ onSelectSession }) => {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [filteredSessions, setFilteredSessions] = useState<Session[]>([]);
  const [dateGroups, setDateGroups] = useState<DateGroup[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [showSkeleton, setShowSkeleton] = useState(true);
  const [showContent, setShowContent] = useState(false);
  const [isInitialLoad, setIsInitialLoad] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchResults, setSearchResults] = useState<{
    count: number;
    currentIndex: number;
  } | null>(null);

  // Edit modal state
  const [showEditModal, setShowEditModal] = useState(false);
  const [editingSession, setEditingSession] = useState<Session | null>(null);

  // Search state for debouncing
  const [searchTerm, setSearchTerm] = useState('');
  const [caseSensitive, setCaseSensitive] = useState(false);
  const debouncedSearchTerm = useDebounce(searchTerm, 300); // 300ms debounce

  const containerRef = useRef<HTMLDivElement>(null);

  const loadSessions = useCallback(async () => {
    setIsLoading(true);
    setShowSkeleton(true);
    setShowContent(false);
    setError(null);
    try {
      const sessions = await fetchSessions();
      // Use startTransition to make state updates non-blocking
      startTransition(() => {
        setSessions(sessions);
        setFilteredSessions(sessions);
      });
    } catch (err) {
      console.error('Failed to load sessions:', err);
      setError('Failed to load sessions. Please try again later.');
      setSessions([]);
      setFilteredSessions([]);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    loadSessions();
  }, [loadSessions]);

  // Timing logic to prevent flicker between skeleton and content on initial load
  useEffect(() => {
    if (!isLoading && showSkeleton) {
      setShowSkeleton(false);
      // Use startTransition for non-blocking content show
      startTransition(() => {
        setTimeout(() => {
          setShowContent(true);
          if (isInitialLoad) {
            setIsInitialLoad(false);
          }
        }, 10);
      });
    }
    return () => void 0;
  }, [isLoading, showSkeleton, isInitialLoad]);

  // Memoize date groups calculation to prevent unnecessary recalculations
  const memoizedDateGroups = useMemo(() => {
    if (filteredSessions.length > 0) {
      return groupSessionsByDate(filteredSessions);
    }
    return [];
  }, [filteredSessions]);

  // Update date groups when filtered sessions change
  useEffect(() => {
    startTransition(() => {
      setDateGroups(memoizedDateGroups);
    });
  }, [memoizedDateGroups]);

  // Debounced search effect - performs actual filtering
  useEffect(() => {
    if (!debouncedSearchTerm) {
      startTransition(() => {
        setFilteredSessions(sessions);
        setSearchResults(null);
      });
      return;
    }

    // Use startTransition to make search non-blocking
    startTransition(() => {
      const searchTerm = caseSensitive ? debouncedSearchTerm : debouncedSearchTerm.toLowerCase();
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
    });
  }, [debouncedSearchTerm, caseSensitive, sessions]);

  // Handle immediate search input (updates search term for debouncing)
  const handleSearch = useCallback((term: string, caseSensitive: boolean) => {
    setSearchTerm(term);
    setCaseSensitive(caseSensitive);
  }, []);

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

  // Handle modal close
  const handleModalClose = useCallback(() => {
    setShowEditModal(false);
    setEditingSession(null);
  }, []);

  const handleModalSave = useCallback(async (sessionId: string, newDescription: string) => {
    // Update state immediately for optimistic UI
    setSessions((prevSessions) =>
      prevSessions.map((s) =>
        s.id === sessionId ? { ...s, metadata: { ...s.metadata, description: newDescription } } : s
      )
    );
  }, []);

  const handleEditSession = useCallback((session: Session) => {
    setEditingSession(session);
    setShowEditModal(true);
  }, []);

  const SessionItem = React.memo(function SessionItem({
    session,
    onEditClick,
  }: {
    session: Session;
    onEditClick: (session: Session) => void;
  }) {
    const handleEditClick = useCallback(
      (e: React.MouseEvent) => {
        e.stopPropagation(); // Prevent card click
        onEditClick(session);
      },
      [onEditClick, session]
    );

    const handleCardClick = useCallback(() => {
      onSelectSession(session.id);
    }, [session.id]);

    return (
      <Card
        onClick={handleCardClick}
        className="session-item h-full py-3 px-4 hover:shadow-default cursor-pointer transition-all duration-150 flex flex-col justify-between relative group"
      >
        <button
          onClick={handleEditClick}
          className="absolute top-3 right-4 p-2 rounded opacity-0 group-hover:opacity-100 transition-opacity hover:bg-gray-100 dark:hover:bg-gray-700 cursor-pointer"
          title="Edit session name"
        >
          <Edit2 className="w-3 h-3 text-textSubtle hover:text-textStandard" />
        </button>

        <div className="flex-1">
          <h3 className="text-base mb-1 pr-6 break-words">
            {session.metadata.description || session.id}
          </h3>

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

  // Render skeleton loader for session items with variations
  const SessionSkeleton = React.memo(({ variant = 0 }: { variant?: number }) => {
    const titleWidths = ['w-3/4', 'w-2/3', 'w-4/5', 'w-1/2'];
    const pathWidths = ['w-32', 'w-28', 'w-36', 'w-24'];
    const tokenWidths = ['w-12', 'w-10', 'w-14', 'w-8'];

    return (
      <Card className="session-skeleton h-full py-3 px-4 flex flex-col justify-between">
        <div className="flex-1">
          <Skeleton className={`h-5 ${titleWidths[variant % titleWidths.length]} mb-2`} />
          <div className="flex items-center mb-1">
            <Skeleton className="h-3 w-3 mr-1 rounded-sm" />
            <Skeleton className="h-4 w-20" />
          </div>
          <div className="flex items-center mb-1">
            <Skeleton className="h-3 w-3 mr-1 rounded-sm" />
            <Skeleton className={`h-4 ${pathWidths[variant % pathWidths.length]}`} />
          </div>
        </div>

        <div className="flex items-center justify-between mt-1 pt-2">
          <div className="flex items-center space-x-3">
            <div className="flex items-center">
              <Skeleton className="h-3 w-3 mr-1 rounded-sm" />
              <Skeleton className="h-4 w-8" />
            </div>
            <div className="flex items-center">
              <Skeleton className="h-3 w-3 mr-1 rounded-sm" />
              <Skeleton className={`h-4 ${tokenWidths[variant % tokenWidths.length]}`} />
            </div>
          </div>
        </div>
      </Card>
    );
  });

  SessionSkeleton.displayName = 'SessionSkeleton';

  const renderActualContent = () => {
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
                <SessionItem key={session.id} session={session} onEditClick={handleEditSession} />
              ))}
            </div>
          </div>
        ))}
      </div>
    );
  };

  return (
    <>
      <MainPanelLayout>
        <div className="flex-1 flex flex-col min-h-0">
          <div className="bg-background-default px-8 pb-8 pt-16">
            <div className="flex flex-col page-transition">
              <div className="flex justify-between items-center mb-1">
                <h1 className="text-4xl font-light">Chat history</h1>
              </div>
              <p className="text-sm text-text-muted mb-4">
                View and search your past conversations with Goose.
              </p>
            </div>
          </div>

          <div className="flex-1 min-h-0 relative px-8">
            <ScrollArea className="h-full" data-search-scroll-area>
              <div ref={containerRef} className="h-full relative">
                <SearchView
                  onSearch={handleSearch}
                  onNavigate={handleSearchNavigation}
                  searchResults={searchResults}
                  className="relative"
                >
                  {/* Skeleton layer - always rendered but conditionally visible */}
                  <div
                    className={`absolute inset-0 transition-opacity duration-300 ${
                      isLoading || showSkeleton
                        ? 'opacity-100 z-10'
                        : 'opacity-0 z-0 pointer-events-none'
                    }`}
                  >
                    <div className="space-y-8">
                      {/* Today section */}
                      <div className="space-y-4">
                        <Skeleton className="h-6 w-16" />
                        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5 gap-4">
                          <SessionSkeleton variant={0} />
                          <SessionSkeleton variant={1} />
                          <SessionSkeleton variant={2} />
                          <SessionSkeleton variant={3} />
                          <SessionSkeleton variant={0} />
                        </div>
                      </div>

                      {/* Yesterday section */}
                      <div className="space-y-4">
                        <Skeleton className="h-6 w-20" />
                        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5 gap-4">
                          <SessionSkeleton variant={1} />
                          <SessionSkeleton variant={2} />
                          <SessionSkeleton variant={3} />
                          <SessionSkeleton variant={0} />
                          <SessionSkeleton variant={1} />
                          <SessionSkeleton variant={2} />
                        </div>
                      </div>

                      {/* Additional section */}
                      <div className="space-y-4">
                        <Skeleton className="h-6 w-24" />
                        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5 gap-4">
                          <SessionSkeleton variant={3} />
                          <SessionSkeleton variant={0} />
                          <SessionSkeleton variant={1} />
                        </div>
                      </div>
                    </div>
                  </div>

                  {/* Content layer - always rendered but conditionally visible */}
                  <div
                    className={`relative transition-opacity duration-300 ${
                      showContent ? 'opacity-100 z-10' : 'opacity-0 z-0'
                    }`}
                  >
                    {renderActualContent()}
                  </div>
                </SearchView>
              </div>
            </ScrollArea>
          </div>
        </div>
      </MainPanelLayout>

      <EditSessionModal
        session={editingSession}
        isOpen={showEditModal}
        onClose={handleModalClose}
        onSave={handleModalSave}
      />
    </>
  );
});

SessionListView.displayName = 'SessionListView';

export default SessionListView;
