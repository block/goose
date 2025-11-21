import React, { useEffect, useState, useRef, useCallback, useMemo, startTransition } from 'react';
import {
  MessageSquareText,
  Target,
  AlertCircle,
  Calendar,
  Folder,
  Edit2,
  Trash2,
  Users,
  Hash,
  MessageCircle,
  RefreshCw,
  Sparkles,
  Grid3X3,
  Clock,
} from 'lucide-react';
import AvatarImage from '../AvatarImage';
import { useMatrix } from '../../contexts/MatrixContext';
import { Card } from '../ui/card';
import { Button } from '../ui/button';
import { ScrollArea } from '../ui/scroll-area';
import { View, ViewOptions } from '../../utils/navigationUtils';
import { formatMessageTimestamp } from '../../utils/timeUtils';
import { SearchView } from '../conversation/SearchView';
import { SearchHighlighter } from '../../utils/searchHighlighter';
import { MainPanelLayout } from '../Layout/MainPanelLayout';
import { groupSessionsByDate, type DateGroup } from '../../utils/dateUtils';
import { Skeleton } from '../ui/skeleton';
import { toast } from 'react-toastify';
import { ConfirmationModal } from '../ui/ConfirmationModal';
import { Session } from '../../api';
import { unifiedSessionService } from '../../services/UnifiedSessionService';
import SessionTimelineView from './SessionTimelineView';

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
        setDescription(session.description || session.id);
      } else if (!isOpen) {
        // Reset state when modal closes
        setDescription('');
        setIsUpdating(false);
      }
    }, [session, isOpen]);

    const handleSave = useCallback(async () => {
      if (!session || disabled) return;

      const trimmedDescription = description.trim();
      if (trimmedDescription === session.description) {
        onClose();
        return;
      }

      setIsUpdating(true);
      try {
        await unifiedSessionService.updateSessionDescriptionById(session.id, trimmedDescription);
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
        setDescription(session.description || session.id);
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
  selectedSessionId?: string | null;
}

const SessionListView: React.FC<SessionListViewProps> = React.memo(
  ({ onSelectSession, selectedSessionId }) => {
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

    // Matrix context for getting participant details
    const { rooms, currentUser } = useMatrix();

    // Edit modal state
    const [showEditModal, setShowEditModal] = useState(false);
    const [editingSession, setEditingSession] = useState<Session | null>(null);

    // Delete confirmation modal state
    const [showDeleteConfirmation, setShowDeleteConfirmation] = useState(false);
    const [sessionToDelete, setSessionToDelete] = useState<Session | null>(null);

    // Search state for debouncing
    const [searchTerm, setSearchTerm] = useState('');
    const [caseSensitive, setCaseSensitive] = useState(false);
    const debouncedSearchTerm = useDebounce(searchTerm, 300); // 300ms debounce

    // View toggle state
    const [viewMode, setViewMode] = useState<'grid' | 'timeline'>('grid');

    const containerRef = useRef<HTMLDivElement>(null);

    // Track session to element ref
    const sessionRefs = useRef<Record<string, HTMLElement>>({});
    const setSessionRefs = (itemId: string, element: HTMLDivElement | null) => {
      if (element) {
        sessionRefs.current[itemId] = element;
      } else {
        delete sessionRefs.current[itemId];
      }
    };

    const loadSessions = useCallback(async () => {
      setIsLoading(true);
      setShowSkeleton(true);
      setShowContent(false);
      setError(null);
      try {
        const resp = await unifiedSessionService.getAllSessions();
        const sessions = resp.sessions;
        console.log('📋 Loaded unified sessions:', {
          total: sessions.length,
          regular: resp.regularSessionCount,
          matrix: resp.matrixSessionCount,
        });
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

    // Scroll to the selected session when returning from session history view
    useEffect(() => {
      if (selectedSessionId) {
        const element = sessionRefs.current[selectedSessionId];
        if (element) {
          element.scrollIntoView({
            block: 'center',
          });
        }
      }
    }, [selectedSessionId, sessions]);

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
          const description = session.description || session.id;
          const workingDir = session.working_dir;
          const sessionId = session.id;

          if (caseSensitive) {
            return (
              description.includes(searchTerm) ||
              sessionId.includes(searchTerm) ||
              workingDir.includes(searchTerm)
            );
          } else {
            return (
              description.toLowerCase().includes(searchTerm) ||
              sessionId.toLowerCase().includes(searchTerm) ||
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
          searchResults.currentIndex === 1
            ? filteredSessions.length
            : searchResults.currentIndex - 1;
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
        prevSessions.map((s) => (s.id === sessionId ? { ...s, description: newDescription } : s))
      );
    }, []);

    const handleEditSession = useCallback((session: Session) => {
      setEditingSession(session);
      setShowEditModal(true);
    }, []);

    const handleDeleteSession = useCallback((session: Session) => {
      setSessionToDelete(session);
      setShowDeleteConfirmation(true);
    }, []);

    const handleConfirmDelete = useCallback(async () => {
      if (!sessionToDelete) return;

      setShowDeleteConfirmation(false);
      const sessionToDeleteId = sessionToDelete.id;
      const sessionName = sessionToDelete.description || sessionToDelete.id;
      setSessionToDelete(null);

      try {
        await unifiedSessionService.deleteSessionById(sessionToDeleteId);
        toast.success('Session deleted successfully');
      } catch (error) {
        console.error('Error deleting session:', error);
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        toast.error(`Failed to delete session "${sessionName}": ${errorMessage}`);
      }
      await loadSessions();
    }, [sessionToDelete, loadSessions]);

    const handleCancelDelete = useCallback(() => {
      setShowDeleteConfirmation(false);
      setSessionToDelete(null);
    }, []);

    // Helper function to get participant details for Matrix sessions
    const getParticipantDetails = useCallback((session: Session) => {
      if (!session.extension_data?.matrix?.roomId) return [];
      
      const roomId = session.extension_data.matrix.roomId;
      const room = rooms.find(r => r.roomId === roomId);
      
      if (!room || !room.members) return [];
      
      // Filter out current user and return participant details
      return room.members
        .filter(member => member.userId !== currentUser?.userId)
        .map(member => ({
          userId: member.userId,
          displayName: member.displayName || member.userId.split(':')[0].substring(1),
          avatarUrl: member.avatarUrl,
        }));
    }, [rooms, currentUser]);

    // Helper function to get recent message participants from conversation
    const getRecentMessageParticipants = useCallback((session: Session) => {
      // TEMPORARILY DISABLED: Message participant display to debug cross-contamination issue
      // The issue appears to be that Matrix sessions are sharing conversation data
      // or there's cross-contamination in the MatrixSessionService
      
      console.log('🚫 Message participants temporarily disabled for debugging');
      return [];
      
      // Only show message participants for Matrix sessions that have conversation data
      // Regular sessions don't load conversation data in the list view for performance
      const isMatrix = session.extension_data?.matrix?.roomId;
      const matrixRoomId = session.extension_data?.matrix?.roomId;
      
      if (!isMatrix || !session.conversation || session.conversation.length === 0) {
        return [];
      }
      
      console.log('🔍 Processing conversation for Matrix session:', {
        sessionId: session.id,
        matrixRoomId,
        hasConversation: !!session.conversation,
        messageCount: session.conversation?.length || 0,
        isMatrix,
        firstFewMessages: session.conversation.slice(0, 3).map(m => ({
          role: m.role,
          contentPreview: m.content?.[0]?.type === 'text' ? m.content[0].text?.substring(0, 50) + '...' : 'non-text',
          created: m.created
        }))
      });
      
      // CRITICAL: Verify this conversation actually belongs to this Matrix room
      // Check if the session ID matches the Matrix room ID or if there's proper mapping
      if (session.id !== matrixRoomId && !session.id.startsWith(matrixRoomId)) {
        console.warn('🚨 Session ID mismatch with Matrix room ID:', {
          sessionId: session.id,
          matrixRoomId,
          description: session.description
        });
        // For now, still process but log the mismatch
      }
      
      // Get unique participants from the last 10 messages
      const recentMessages = session.conversation.slice(-10);
      const participantMap = new Map<string, { role: string; count: number; lastSeen: number }>();
      
      recentMessages.forEach((message, index) => {
        if (message.role && message.role !== 'system') {
          const existing = participantMap.get(message.role);
          if (existing) {
            existing.count += 1;
            existing.lastSeen = index;
          } else {
            participantMap.set(message.role, { role: message.role, count: 1, lastSeen: index });
          }
        }
      });
      
      // Sort by last seen (most recent first) and return formatted list
      const participants = Array.from(participantMap.entries())
        .sort(([, a], [, b]) => b.lastSeen - a.lastSeen)
        .map(([role, data]) => ({
          role,
          count: data.count,
          displayName: role === 'user' ? 'You' : role === 'assistant' ? 'Goose' : role,
          isUser: role === 'user',
          isAssistant: role === 'assistant',
        }));
      
      console.log('🔍 Found participants for session:', session.id, participants);
      return participants;
    }, []);

    // Component to display participant avatars
    const ParticipantAvatars = React.memo(({ session }: { session: Session }) => {
      const participants = getParticipantDetails(session);
      
      if (participants.length === 0) return null;
      
      // For DMs (1 other participant), show single avatar
      if (participants.length === 1) {
        const participant = participants[0];
        return (
          <div className="flex items-center gap-1">
            <AvatarImage
              avatarUrl={participant.avatarUrl}
              displayName={participant.displayName}
              size="sm"
              className="ring-1 ring-border-subtle"
            />
          </div>
        );
      }
      
      // For group chats, show overlapping avatars (max 3 + counter)
      const visibleParticipants = participants.slice(0, 3);
      const remainingCount = participants.length - 3;
      
      return (
        <div className="flex items-center">
          <div className="flex -space-x-1">
            {visibleParticipants.map((participant) => (
              <AvatarImage
                key={participant.userId}
                avatarUrl={participant.avatarUrl}
                displayName={participant.displayName}
                size="sm"
                className="ring-2 ring-background-default"
              />
            ))}
            {remainingCount > 0 && (
              <div className="w-6 h-6 rounded-full bg-background-accent flex items-center justify-center ring-2 ring-background-default">
                <span className="text-xs font-medium text-text-on-accent">
                  +{remainingCount}
                </span>
              </div>
            )}
          </div>
        </div>
      );
    });

    ParticipantAvatars.displayName = 'ParticipantAvatars';

    // Component to display recent message participants with role indicators
    const RecentMessageParticipants = React.memo(({ session }: { session: Session }) => {
      const participants = getRecentMessageParticipants(session);
      
      if (participants.length === 0) return null;
      
      return (
        <div className="flex items-center gap-1 text-xs">
          {participants.slice(0, 3).map((participant, index) => (
            <div
              key={participant.role}
              className={`flex items-center gap-1 px-2 py-1 rounded-full ${
                participant.isUser
                  ? 'bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300'
                  : participant.isAssistant
                  ? 'bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300'
                  : 'bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300'
              }`}
              title={`${participant.displayName}: ${participant.count} message${participant.count > 1 ? 's' : ''}`}
            >
              <span className="font-medium">{participant.displayName}</span>
              {participant.count > 1 && (
                <span className="text-xs opacity-75">×{participant.count}</span>
              )}
            </div>
          ))}
          {participants.length > 3 && (
            <div className="text-xs text-text-muted">
              +{participants.length - 3} more
            </div>
          )}
        </div>
      );
    });

    RecentMessageParticipants.displayName = 'RecentMessageParticipants';

    const SessionItem = React.memo(function SessionItem({
      session,
      onEditClick,
      onDeleteClick,
    }: {
      session: Session;
      onEditClick: (session: Session) => void;
      onDeleteClick: (session: Session) => void;
    }) {
      const [isRegeneratingTitle, setIsRegeneratingTitle] = useState(false);
      const handleEditClick = useCallback(
        (e: React.MouseEvent) => {
          e.stopPropagation(); // Prevent card click
          onEditClick(session);
        },
        [onEditClick, session]
      );

      const handleDeleteClick = useCallback(
        (e: React.MouseEvent) => {
          e.stopPropagation(); // Prevent card click
          onDeleteClick(session);
        },
        [onDeleteClick, session]
      );

      const handleCardClick = useCallback(() => {
        onSelectSession(session.id);
      }, [session.id]);

      const handleRegenerateTitle = useCallback(async (e: React.MouseEvent) => {
        e.stopPropagation(); // Prevent card click
        
        if (isRegeneratingTitle) return;
        
        setIsRegeneratingTitle(true);
        try {
          const newTitle = await unifiedSessionService.regenerateSessionTitle(session.id);
          if (newTitle) {
            // Update the session in the local state
            setSessions(prevSessions =>
              prevSessions.map(s => 
                s.id === session.id 
                  ? { ...s, description: newTitle }
                  : s
              )
            );
            toast.success('Session title regenerated successfully');
          } else {
            toast.error('Failed to regenerate title - this feature is only available for Matrix sessions');
          }
        } catch (error) {
          console.error('Error regenerating title:', error);
          toast.error('Failed to regenerate session title');
        } finally {
          setIsRegeneratingTitle(false);
        }
      }, [session.id, isRegeneratingTitle]);

      // Get session display info
      const displayInfo = unifiedSessionService.getSessionDisplayInfo(session);
      const isMatrix = displayInfo.type === 'matrix' || displayInfo.type === 'collaborative';
      const isMatrixDM = isMatrix && session.extension_data?.matrix?.isDirectMessage;
      const isCollaborative = displayInfo.type === 'collaborative';

      // Enhanced styling for collaborative sessions
      const borderStyle = isCollaborative 
        ? 'border-l-4 border-l-purple-500 dark:border-l-purple-400' 
        : '';
      const bgStyle = isCollaborative 
        ? 'bg-gradient-to-r from-purple-50/50 to-transparent dark:from-purple-900/20 dark:to-transparent' 
        : '';

      return (
        <Card
          onClick={handleCardClick}
          className={`session-item h-full py-3 px-4 hover:shadow-default cursor-pointer transition-all duration-150 flex flex-col justify-between relative group ${borderStyle} ${bgStyle}`}
          ref={(el) => setSessionRefs(session.id, el)}
        >
          {/* Session type icon in uppermost right corner */}
          <div className="absolute top-3 right-3">
            {isMatrix && (
              <div className="flex items-center gap-1">
                {isMatrixDM ? (
                  <MessageCircle className="w-4 h-4 text-green-500" title="Matrix Direct Message" />
                ) : displayInfo.type === 'collaborative' ? (
                  <Users className="w-4 h-4 text-purple-500" title="Collaborative Matrix Session" />
                ) : (
                  <Hash className="w-4 h-4 text-blue-500" title="Matrix Group Chat" />
                )}
              </div>
            )}
          </div>

          {/* Action buttons on hover */}
          <div className="absolute top-3 right-8 flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
            {isMatrix && (
              <button
                onClick={handleRegenerateTitle}
                disabled={isRegeneratingTitle}
                className="p-2 rounded hover:bg-blue-50 dark:hover:bg-blue-900/20 cursor-pointer transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                title="Regenerate title using AI"
              >
                {isRegeneratingTitle ? (
                  <RefreshCw className="w-3 h-3 text-blue-500 animate-spin" />
                ) : (
                  <Sparkles className="w-3 h-3 text-blue-500 hover:text-blue-600" />
                )}
              </button>
            )}
            <button
              onClick={handleEditClick}
              className="p-2 rounded hover:bg-gray-100 dark:hover:bg-gray-700 cursor-pointer"
              title="Edit session name"
            >
              <Edit2 className="w-3 h-3 text-textSubtle hover:text-textStandard" />
            </button>
            <button
              onClick={handleDeleteClick}
              className="p-2 rounded hover:bg-red-50 dark:hover:bg-red-900/20 cursor-pointer transition-colors"
              title="Delete session"
            >
              <Trash2 className="w-3 h-3 text-red-500 hover:text-red-600" />
            </button>
          </div>

          <div className="flex-1">
            <div className="flex items-center gap-2 mb-1 pr-16">
              <h3 className="text-base break-words flex-1">
                {session.description || session.id}
              </h3>
              {/* Collaborative session badge */}
              {isCollaborative && (
                <div className="flex items-center gap-1 px-2 py-1 bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-300 rounded-full text-xs font-medium">
                  <Users className="w-3 h-3" />
                  <span>Collaborative</span>
                </div>
              )}
            </div>

            <div className="flex items-center text-text-muted text-xs mb-1">
              <Calendar className="w-3 h-3 mr-1 flex-shrink-0" />
              <span>{formatMessageTimestamp(Date.parse(session.updated_at) / 1000)}</span>
            </div>
            <div className="flex items-center text-text-muted text-xs mb-1">
              <Folder className="w-3 h-3 mr-1 flex-shrink-0" />
              <span className="truncate">{displayInfo.workingDir}</span>
            </div>
            {isMatrix && displayInfo.participants && (
              <div className="flex items-center text-text-muted text-xs mb-1">
                <Users className="w-3 h-3 mr-1 flex-shrink-0" />
                <span className="truncate">{displayInfo.participants.length} participants</span>
              </div>
            )}
            
            {/* Show recent message participants for all sessions */}
            <div className="mb-1">
              <RecentMessageParticipants session={session} />
            </div>
          </div>

          <div className="flex items-center justify-between mt-1 pt-2">
            <div className="flex items-center space-x-3 text-xs text-text-muted">
              <div className="flex items-center">
                <MessageSquareText className="w-3 h-3 mr-1" />
                <span className="font-mono">{session.message_count}</span>
              </div>
              {displayInfo.hasTokenCounts && session.total_tokens !== null && (
                <div className="flex items-center">
                  <Target className="w-3 h-3 mr-1" />
                  <span className="font-mono">{(session.total_tokens || 0).toLocaleString()}</span>
                </div>
              )}
            </div>
            {isMatrix && (
              <ParticipantAvatars session={session} />
            )}
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

      if (filteredSessions.length === 0 && searchResults !== null) {
        return (
          <div className="flex flex-col items-center justify-center h-full text-text-muted mt-4">
            <MessageSquareText className="h-12 w-12 mb-4" />
            <p className="text-lg mb-2">No matching sessions found</p>
            <p className="text-sm">Try adjusting your search terms</p>
          </div>
        );
      }

      // Render timeline view
      if (viewMode === 'timeline') {
        return (
          <SessionTimelineView
            sessions={filteredSessions}
            onSelectSession={onSelectSession}
            selectedSessionId={selectedSessionId}
            className="h-full"
          />
        );
      }

      // Render grid view (default)
      if (dateGroups.length === 0) {
        return (
          <div className="flex flex-col items-center justify-center h-full text-text-muted mt-4">
            <MessageSquareText className="h-12 w-12 mb-4" />
            <p className="text-lg mb-2">No sessions to display</p>
            <p className="text-sm">Your chat history will appear here</p>
          </div>
        );
      }

      return (
        <div className="space-y-8">
          {dateGroups.map((group) => (
            <div key={group.label} className="space-y-4">
              <div className="sticky top-0 z-10 bg-background-default/95 backdrop-blur-sm">
                <h2 className="text-text-muted">{group.label}</h2>
              </div>
              <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5 gap-4">
                {group.sessions.map((session) => (
                  <SessionItem
                    key={session.id}
                    session={session}
                    onEditClick={handleEditSession}
                    onDeleteClick={handleDeleteSession}
                  />
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
                  
                  {/* View toggle buttons */}
                  <div className="flex items-center gap-2 bg-gray-100 dark:bg-gray-800 rounded-lg p-1">
                    <Button
                      variant={viewMode === 'grid' ? 'default' : 'ghost'}
                      size="sm"
                      onClick={() => setViewMode('grid')}
                      className="px-3 py-1.5 h-auto"
                    >
                      <Grid3X3 className="w-4 h-4 mr-2" />
                      Grid
                    </Button>
                    <Button
                      variant={viewMode === 'timeline' ? 'default' : 'ghost'}
                      size="sm"
                      onClick={() => setViewMode('timeline')}
                      className="px-3 py-1.5 h-auto"
                    >
                      <Clock className="w-4 h-4 mr-2" />
                      Timeline
                    </Button>
                  </div>
                </div>
                <p className="text-sm text-text-muted mb-4">
                  {viewMode === 'timeline' 
                    ? 'View your chat sessions on a visual timeline showing when they started and ended.'
                    : 'View and search your past conversations with Goose.'
                  }
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

        <ConfirmationModal
          isOpen={showDeleteConfirmation}
          title="Delete Session"
          message={`Are you sure you want to delete the session "${sessionToDelete?.description || sessionToDelete?.id}"? This action cannot be undone.`}
          confirmLabel="Delete Session"
          cancelLabel="Cancel"
          confirmVariant="destructive"
          onConfirm={handleConfirmDelete}
          onCancel={handleCancelDelete}
        />
      </>
    );
  }
);

SessionListView.displayName = 'SessionListView';

export default SessionListView;
