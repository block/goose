import React, { useState, useEffect, useCallback, useRef } from 'react';
import { useNavigate, useLocation, useSearchParams } from 'react-router-dom';
import {
  Home,
  MessageSquare,
  History,
  FileText,
  Clock,
  Puzzle,
  Settings,
  GripVertical,
  Plus,
} from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';
import { useNavigationContext } from './NavigationContext';
import { cn } from '../../utils';
import { listSessions, getSessionInsights } from '../../api';
import { useConfig } from '../ConfigContext';
import { useChatContext } from '../../contexts/ChatContext';
import { useNavigation } from '../../hooks/useNavigation';
import { startNewSession, resumeSession, shouldShowNewChatTitle } from '../../sessions';
import { getInitialWorkingDir } from '../../utils/workingDir';
import { useSidebarSessionStatus } from '../../hooks/useSidebarSessionStatus';
import { SessionIndicators } from '../SessionIndicators';
import { AppEvents } from '../../constants/events';
import type { Session } from '../../api';
import type { UserInput } from '../../types/message';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '../ui/dropdown-menu';

interface NavItem {
  id: string;
  path: string;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  getTag?: () => string;
  tagAlign?: 'left' | 'right';
  hasSubItems?: boolean;
}

interface ActiveSession {
  sessionId: string;
  initialMessage?: UserInput;
}

interface ExpandedNavigationProps {
  className?: string;
  activeSessions?: ActiveSession[];
}

export const ExpandedNavigation: React.FC<ExpandedNavigationProps> = ({
  className,
  activeSessions = [],
}) => {
  const navigate = useNavigate();
  const location = useLocation();
  const { extensionsList } = useConfig();
  const {
    isNavExpanded,
    setIsNavExpanded,
    effectiveNavigationMode,
    navigationPosition,
    preferences,
    updatePreferences,
  } = useNavigationContext();

  const [draggedItem, setDraggedItem] = useState<string | null>(null);
  const [dragOverItem, setDragOverItem] = useState<string | null>(null);
  const [chatDropdownOpen, setChatDropdownOpen] = useState(false);
  const [recentSessions, setRecentSessions] = useState<Session[]>([]);
  const [gridColumns, setGridColumns] = useState(2);
  const [gridMeasured, setGridMeasured] = useState(false);
  const [tilesReady, setTilesReady] = useState(false);
  const [isClosing, setIsClosing] = useState(false);
  const prevIsNavExpandedRef = useRef(isNavExpanded);
  const gridRef = useRef<HTMLDivElement>(null);

  // Stats for tags
  const [currentTime, setCurrentTime] = useState('');
  const [todayChatsCount, setTodayChatsCount] = useState(0);
  const [totalSessions, setTotalSessions] = useState(0);
  const [recipesCount] = useState(0); // TODO: Fetch actual recipes count

  // Update time
  useEffect(() => {
    const updateTime = () => {
      const now = new Date();
      setCurrentTime(
        now.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: true })
      );
    };
    updateTime();
    const interval = setInterval(updateTime, 1000);
    return () => clearInterval(interval);
  }, []);

  // Ref to track sessions for new chat logic without causing re-renders
  const sessionsRef = useRef<Session[]>([]);

  // Guard ref to prevent duplicate session creation
  const isCreatingSessionRef = useRef(false);

  // Update sessions ref when we fetch navigation data
  const fetchNavigationData = async () => {
    try {
      const sessionsResponse = await listSessions({ throwOnError: false });
      if (sessionsResponse.data) {
        const today = new Date();
        today.setHours(0, 0, 0, 0);

        let todayCount = 0;
        sessionsResponse.data.sessions.forEach((session) => {
          const sessionDate = new Date(session.created_at);
          sessionDate.setHours(0, 0, 0, 0);
          if (sessionDate.getTime() === today.getTime()) {
            todayCount++;
          }
        });

        setTodayChatsCount(todayCount);

        // Store sessions for new chat logic
        sessionsRef.current = sessionsResponse.data.sessions;

        // Get recent sessions for dropdown (sorted by most recent, limit 10)
        const sortedSessions = [...sessionsResponse.data.sessions]
          .sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())
          .slice(0, 10);
        setRecentSessions(sortedSessions);
      }

      const insightsResponse = await getSessionInsights({ throwOnError: false });
      if (insightsResponse.data) {
        setTotalSessions(insightsResponse.data.totalSessions || 0);
      }
    } catch (error) {
      console.error('Failed to fetch navigation data:', error);
    }
  };

  // Fetch stats when expanded
  useEffect(() => {
    if (isNavExpanded) {
      fetchNavigationData();
    }
  }, [isNavExpanded]);

  // Detect when nav is closing (transition from expanded to collapsed)
  useEffect(() => {
    if (prevIsNavExpandedRef.current && !isNavExpanded) {
      // Nav is closing - immediately hide tiles to prevent layout thrashing
      setIsClosing(true);
      setTilesReady(false);
    } else if (!prevIsNavExpandedRef.current && isNavExpanded) {
      // Nav is opening - reset closing state
      setIsClosing(false);
    }
    prevIsNavExpandedRef.current = isNavExpanded;
  }, [isNavExpanded]);

  // Control when tiles are ready to animate in (after panel opens)
  useEffect(() => {
    if (!isNavExpanded) {
      setTilesReady(false);
      return;
    }

    // Delay tile animations until panel has opened (give it ~200ms)
    const timeoutId = setTimeout(() => {
      setTilesReady(true);
    }, 150);

    return () => clearTimeout(timeoutId);
  }, [isNavExpanded]);

  // Track grid columns for spacer tiles
  useEffect(() => {
    // Reset measured state when nav closes
    if (!isNavExpanded) {
      setGridMeasured(false);
      return;
    }

    const updateGridColumns = () => {
      if (gridRef.current) {
        const gridStyle = window.getComputedStyle(gridRef.current);
        const columns = gridStyle.gridTemplateColumns
          .split(' ')
          .filter((col) => col.trim() !== '').length;
        if (columns > 0) {
          setGridColumns(columns);
          setGridMeasured(true);
        }
      }
    };

    // Initial update with a small delay to ensure grid is rendered
    const timeoutId = setTimeout(updateGridColumns, 100);

    // Use ResizeObserver for more reliable updates
    const resizeObserver = new ResizeObserver(() => {
      updateGridColumns();
    });

    if (gridRef.current) {
      resizeObserver.observe(gridRef.current);
    }

    window.addEventListener('resize', updateGridColumns);

    return () => {
      clearTimeout(timeoutId);
      resizeObserver.disconnect();
      window.removeEventListener('resize', updateGridColumns);
    };
  }, [isNavExpanded, navigationPosition]);

  // Build nav items with dynamic tags
  const getNavItems = (): NavItem[] => [
    {
      id: 'home',
      path: '/',
      label: 'Home',
      icon: Home,
      getTag: () => currentTime,
    },
    {
      id: 'chat',
      path: '/pair',
      label: 'Chat',
      icon: MessageSquare,
      getTag: () =>
        activeSessions.length > 0 ? `${activeSessions.length} active` : `${todayChatsCount} today`,
      tagAlign: 'right',
      hasSubItems: activeSessions.length > 0,
    },
    {
      id: 'history',
      path: '/sessions',
      label: 'History',
      icon: History,
      getTag: () => `${totalSessions}`,
      tagAlign: 'right',
    },
    {
      id: 'recipes',
      path: '/recipes',
      label: 'Recipes',
      icon: FileText,
      getTag: () => `${recipesCount}`,
    },
    {
      id: 'scheduler',
      path: '/schedules',
      label: 'Scheduler',
      icon: Clock,
    },
    {
      id: 'extensions',
      path: '/extensions',
      label: 'Extensions',
      icon: Puzzle,
      getTag: () => {
        if (!extensionsList || !Array.isArray(extensionsList)) return '0/0';
        const enabled = extensionsList.filter((ext) => ext.enabled).length;
        return `${enabled}/${extensionsList.length}`;
      },
    },
    {
      id: 'settings',
      path: '/settings',
      label: 'Settings',
      icon: Settings,
    },
  ];

  const navItems = getNavItems();

  const getNavItemById = (id: string): NavItem | undefined => {
    return navItems.find((item) => item.id === id);
  };

  // Handle escape key to close overlay
  useEffect(() => {
    if (!(effectiveNavigationMode === 'overlay' && isNavExpanded)) {
      return;
    }

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isNavExpanded && effectiveNavigationMode === 'overlay') {
        e.preventDefault();
        setIsNavExpanded(false);
      }
    };

    document.addEventListener('keydown', handleKeyDown, { capture: true });
    return () => document.removeEventListener('keydown', handleKeyDown, { capture: true });
  }, [isNavExpanded, effectiveNavigationMode, setIsNavExpanded]);

  // Track session for /pair navigation
  const [searchParams] = useSearchParams();
  const chatContext = useChatContext();
  const lastSessionIdRef = useRef<string | null>(null);
  const activeSessionId = searchParams.get('resumeSessionId') ?? undefined;
  const currentSessionId =
    location.pathname === '/pair' ? searchParams.get('resumeSessionId') : null;

  // Use sidebar session status hook for streaming/unread indicators
  const { getSessionStatus, clearUnread } = useSidebarSessionStatus(activeSessionId);

  // Use setView for navigation to pair view (matches original AppSidebar behavior)
  const setView = useNavigation();

  // Keep track of last session ID
  useEffect(() => {
    if (currentSessionId) {
      lastSessionIdRef.current = currentSessionId;
    }
  }, [currentSessionId]);

  // Listen for new session creation events to update the list immediately
  useEffect(() => {
    let pollingTimeouts: ReturnType<typeof setTimeout>[] = [];
    let isPolling = false;

    const handleSessionCreated = (event: Event) => {
      const { session } = (event as CustomEvent<{ session?: Session }>).detail || {};
      // If session data is provided, add it immediately
      if (session) {
        setRecentSessions((prev) => {
          if (prev.some((s) => s.id === session.id)) return prev;
          return [session, ...prev].slice(0, 10);
        });
        sessionsRef.current = [session, ...sessionsRef.current.filter((s) => s.id !== session.id)];
      }

      // Poll for updates to get the generated session name
      if (isPolling) return;
      isPolling = true;

      const pollIntervalMs = 300;
      const maxPollDurationMs = 10000;
      const maxPolls = maxPollDurationMs / pollIntervalMs;
      let pollCount = 0;

      const pollForUpdates = async () => {
        pollCount++;
        try {
          const response = await listSessions({ throwOnError: false });
          if (response.data) {
            const apiSessions = response.data.sessions.slice(0, 10);
            setRecentSessions((prev) => {
              // Merge: keep empty local sessions not in API, add API sessions
              const emptyLocalSessions = prev.filter(
                (local) =>
                  local.message_count === 0 && !apiSessions.some((api) => api.id === local.id)
              );
              return [...emptyLocalSessions, ...apiSessions].slice(0, 10);
            });
            sessionsRef.current = response.data.sessions;
          }
        } catch (error) {
          console.error('Failed to poll sessions:', error);
        }

        if (pollCount < maxPolls) {
          const timeout = setTimeout(pollForUpdates, pollIntervalMs);
          pollingTimeouts.push(timeout);
        } else {
          isPolling = false;
        }
      };

      pollForUpdates();
    };

    window.addEventListener(AppEvents.SESSION_CREATED, handleSessionCreated);
    return () => {
      window.removeEventListener(AppEvents.SESSION_CREATED, handleSessionCreated);
      pollingTimeouts.forEach(clearTimeout);
    };
  }, []);

  const handleNavClick = useCallback(
    (path: string) => {
      // For /pair, preserve the current session if one exists
      if (path === '/pair') {
        const sessionId =
          currentSessionId || lastSessionIdRef.current || chatContext?.chat?.sessionId;
        if (sessionId && sessionId.length > 0) {
          navigate(`/pair?resumeSessionId=${sessionId}`);
        } else {
          // No session - go to home to start a new chat
          navigate('/');
        }
      } else {
        navigate(path);
      }
      // Close nav on selection only for overlay mode
      if (effectiveNavigationMode === 'overlay') {
        setIsNavExpanded(false);
      }
    },
    [
      navigate,
      currentSessionId,
      chatContext?.chat?.sessionId,
      effectiveNavigationMode,
      setIsNavExpanded,
    ]
  );

  // New chat handler - matches original AppSidebar implementation
  // If there's already an empty session, resume it; otherwise create a new one
  const handleNewChat = useCallback(async () => {
    if (isCreatingSessionRef.current) {
      return;
    }

    // Check if there's already an empty "New Chat" session we can reuse
    const emptyNewSession = sessionsRef.current.find((s) => shouldShowNewChatTitle(s));

    if (emptyNewSession) {
      // Resume the existing empty session
      resumeSession(emptyNewSession, setView);
    } else {
      // Create a new session
      isCreatingSessionRef.current = true;
      try {
        await startNewSession('', setView, getInitialWorkingDir());
      } finally {
        setTimeout(() => {
          isCreatingSessionRef.current = false;
        }, 1000);
      }
    }
    // Close nav on selection only for overlay mode
    if (effectiveNavigationMode === 'overlay') {
      setIsNavExpanded(false);
    }
  }, [setView, effectiveNavigationMode, setIsNavExpanded]);

  const handleSessionClick = useCallback(
    (sessionId: string) => {
      navigate(`/pair?resumeSessionId=${sessionId}`);
      // Close nav on selection only for overlay mode
      if (effectiveNavigationMode === 'overlay') {
        setIsNavExpanded(false);
      }
    },
    [navigate, effectiveNavigationMode, setIsNavExpanded]
  );

  const isActive = (path: string) => location.pathname === path;

  // Drag and drop handlers
  const handleDragStart = (e: React.DragEvent, itemId: string) => {
    setDraggedItem(itemId);
    e.dataTransfer.effectAllowed = 'move';
  };

  const handleDragOver = (e: React.DragEvent, itemId: string) => {
    e.preventDefault();
    if (draggedItem && draggedItem !== itemId) {
      setDragOverItem(itemId);
    }
  };

  const handleDrop = (e: React.DragEvent, dropItemId: string) => {
    e.preventDefault();
    if (!draggedItem || draggedItem === dropItemId) return;

    const newOrder = [...preferences.itemOrder];
    const draggedIndex = newOrder.indexOf(draggedItem);
    const dropIndex = newOrder.indexOf(dropItemId);

    if (draggedIndex === -1 || dropIndex === -1) return;

    newOrder.splice(draggedIndex, 1);
    newOrder.splice(dropIndex, 0, draggedItem);

    updatePreferences({
      ...preferences,
      itemOrder: newOrder,
    });

    setDraggedItem(null);
    setDragOverItem(null);
  };

  const handleDragEnd = () => {
    setDraggedItem(null);
    setDragOverItem(null);
  };

  // Get ordered and enabled items
  const visibleItems = preferences.itemOrder
    .filter((id) => preferences.enabledItems.includes(id))
    .map((id) => getNavItemById(id))
    .filter((item): item is NavItem => item !== undefined);

  const isOverlayMode = effectiveNavigationMode === 'overlay';

  // Truncate session message for display
  const truncateMessage = (msg?: string, maxLen = 24) => {
    if (!msg) return 'New Chat';
    return msg.length > maxLen ? msg.substring(0, maxLen) + '...' : msg;
  };

  // Determine if content should be visible (not during close animation for push mode)
  const showContent = !isClosing || isOverlayMode;

  const navContent = (
    <motion.div
      initial={{ opacity: 0, scale: 0.95 }}
      animate={{ opacity: 1, scale: 1 }}
      exit={{ opacity: 0, scale: 0.95 }}
      transition={{ type: 'spring', stiffness: 350, damping: 25 }}
      className={cn(
        'bg-app h-full overflow-hidden',
        isOverlayMode && 'backdrop-blur-md shadow-2xl rounded-lg p-4',
        // Add 2px padding on the edge facing the content (push mode only)
        !isOverlayMode && navigationPosition === 'top' && 'pb-[2px]',
        !isOverlayMode && navigationPosition === 'bottom' && 'pt-[2px]',
        !isOverlayMode && navigationPosition === 'left' && 'pr-[2px]',
        !isOverlayMode && navigationPosition === 'right' && 'pl-[2px]',
        className
      )}
    >
      {/* Navigation grid - square tiles with scroll */}
      {/* Completely hide content during close animation to prevent layout thrashing */}
      {showContent ? (
        <div
          ref={gridRef}
          className={cn(
            'grid gap-[2px] overflow-y-auto overflow-x-hidden h-full',
            isOverlayMode && 'gap-3'
          )}
          style={{
            // Use CSS grid with auto-fill for responsive tiles based on container width
            gridTemplateColumns: isOverlayMode
              ? // For overlay mode: responsive - single row on larger screens, wraps to 2 rows on smaller
                'repeat(auto-fit, minmax(120px, 1fr))'
              : navigationPosition === 'left' || navigationPosition === 'right'
                ? // For left/right: larger min size (140px) to trigger single column sooner
                  'repeat(auto-fill, minmax(140px, 1fr))'
                : // For top/bottom: auto-fit with larger min size to fit all in 1 row on large screens, wrap to 2 rows on smaller
                  'repeat(auto-fit, minmax(160px, 1fr))',
            // Align items to start so they don't stretch vertically
            alignContent: 'start',
          }}
        >
          {visibleItems.map((item, index) => {
            const Icon = item.icon;
            const active = isActive(item.path);
            const isDragging = draggedItem === item.id;
            const isDragOver = dragOverItem === item.id;
            const isChatItem = item.id === 'chat';

            // Chat tile with dropdown
            if (isChatItem) {
              return (
                <DropdownMenu
                  key={item.id}
                  open={chatDropdownOpen}
                  onOpenChange={setChatDropdownOpen}
                >
                  <motion.div
                    draggable
                    onDragStart={(e) => handleDragStart(e as unknown as React.DragEvent, item.id)}
                    onDragOver={(e) => handleDragOver(e as unknown as React.DragEvent, item.id)}
                    onDrop={(e) => handleDrop(e as unknown as React.DragEvent, item.id)}
                    onDragEnd={handleDragEnd}
                    initial={{ opacity: 0, y: 20, scale: 0.9 }}
                    animate={{
                      opacity: tilesReady ? (isDragging ? 0.5 : 1) : 0,
                      y: tilesReady ? 0 : 20,
                      scale: tilesReady ? (isDragging ? 0.95 : 1) : 0.9,
                    }}
                    transition={{
                      type: 'spring',
                      stiffness: 350,
                      damping: 25,
                      delay: tilesReady ? index * 0.05 : 0,
                    }}
                    className={cn(
                      'relative cursor-move group',
                      isDragOver && 'ring-2 ring-blue-500 rounded-lg'
                    )}
                  >
                    <div className="relative">
                      <DropdownMenuTrigger asChild>
                        <motion.div
                          className={cn(
                            'w-full relative flex flex-col',
                            'rounded-lg',
                            'transition-colors duration-200',
                            'aspect-square cursor-pointer',
                            active
                              ? 'bg-background-accent text-text-on-accent'
                              : 'bg-background-default hover:bg-background-medium'
                          )}
                        >
                          <div className="flex-1 flex flex-col items-start justify-between p-5 no-drag text-left">
                            {/* Drag handle */}
                            <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity z-10">
                              <GripVertical className="w-4 h-4 text-text-muted" />
                            </div>

                            {/* Tag/Badge */}
                            {item.getTag && (
                              <div
                                className={cn(
                                  'absolute top-3 px-2 py-1 rounded-full',
                                  item.tagAlign === 'left' ? 'left-8' : 'right-8',
                                  'bg-background-muted'
                                )}
                              >
                                <span className="text-xs font-mono text-text-muted">
                                  {item.getTag()}
                                </span>
                              </div>
                            )}

                            {/* Icon and Label at bottom */}
                            <div className="mt-auto w-full">
                              <Icon className="w-6 h-6 mb-2" />
                              <h2 className="font-light text-left text-xl">{item.label}</h2>
                            </div>
                          </div>
                        </motion.div>
                      </DropdownMenuTrigger>

                      {/* New Chat button - bottom right corner, outside DropdownMenuTrigger */}
                      <motion.button
                        onClick={(e) => {
                          e.stopPropagation();
                          e.preventDefault();
                          handleNewChat();
                        }}
                        whileHover={{ scale: 1.1 }}
                        whileTap={{ scale: 0.95 }}
                        className={cn(
                          'absolute bottom-3 right-3 p-2 rounded-md z-10',
                          'opacity-0 group-hover:opacity-100 transition-opacity',
                          active
                            ? 'bg-background-default/20 hover:bg-background-default/30 text-text-on-accent'
                            : 'bg-background-medium hover:bg-background-accent hover:text-text-on-accent',
                          'flex items-center justify-center'
                        )}
                        title="New Chat"
                      >
                        <Plus className="w-4 h-4" />
                      </motion.button>
                    </div>
                    <DropdownMenuContent
                      className="w-64 p-1 bg-background-default border-border-subtle rounded-lg shadow-lg z-[10001]"
                      side="right"
                      align="start"
                      sideOffset={8}
                    >
                      {/* New chat button */}
                      <DropdownMenuItem
                        onClick={handleNewChat}
                        className="flex items-center gap-2 px-3 py-2 text-sm rounded-lg cursor-pointer"
                      >
                        <Plus className="w-4 h-4 flex-shrink-0" />
                        <span>New Chat</span>
                      </DropdownMenuItem>

                      {recentSessions.length > 0 && <DropdownMenuSeparator className="my-1" />}

                      {/* Recent sessions */}
                      {recentSessions.map((session) => {
                        const status = getSessionStatus(session.id);
                        const isStreaming = status?.streamState === 'streaming';
                        const hasError = status?.streamState === 'error';
                        const hasUnread = status?.hasUnreadActivity ?? false;
                        return (
                          <DropdownMenuItem
                            key={session.id}
                            onClick={() => {
                              clearUnread(session.id);
                              handleSessionClick(session.id);
                            }}
                            className="flex items-center gap-2 px-3 py-2 text-sm rounded-lg cursor-pointer"
                          >
                            <MessageSquare className="w-4 h-4 flex-shrink-0 text-text-muted" />
                            <span className="truncate flex-1">
                              {truncateMessage(session.name, 30)}
                            </span>
                            <SessionIndicators
                              isStreaming={isStreaming}
                              hasUnread={hasUnread}
                              hasError={hasError}
                            />
                          </DropdownMenuItem>
                        );
                      })}

                      {/* Show All button */}
                      {totalSessions > 10 && (
                        <>
                          <DropdownMenuSeparator className="my-1" />
                          <DropdownMenuItem
                            onClick={() => handleNavClick('/sessions')}
                            className="flex items-center gap-2 px-3 py-2 text-sm rounded-lg cursor-pointer text-text-muted"
                          >
                            <History className="w-4 h-4 flex-shrink-0" />
                            <span>Show All ({totalSessions})</span>
                          </DropdownMenuItem>
                        </>
                      )}
                    </DropdownMenuContent>
                  </motion.div>
                </DropdownMenu>
              );
            }

            // Regular tile for non-chat items
            return (
              <motion.div
                key={item.id}
                draggable
                onDragStart={(e) => handleDragStart(e as unknown as React.DragEvent, item.id)}
                onDragOver={(e) => handleDragOver(e as unknown as React.DragEvent, item.id)}
                onDrop={(e) => handleDrop(e as unknown as React.DragEvent, item.id)}
                onDragEnd={handleDragEnd}
                initial={{ opacity: 0, y: 20, scale: 0.9 }}
                animate={{
                  opacity: tilesReady ? (isDragging ? 0.5 : 1) : 0,
                  y: tilesReady ? 0 : 20,
                  scale: tilesReady ? (isDragging ? 0.95 : 1) : 0.9,
                }}
                transition={{
                  type: 'spring',
                  stiffness: 350,
                  damping: 25,
                  delay: tilesReady ? index * 0.05 : 0,
                }}
                className={cn(
                  'relative cursor-move group',
                  isDragOver && 'ring-2 ring-blue-500 rounded-lg'
                )}
              >
                <motion.div
                  className={cn(
                    'w-full relative flex flex-col',
                    'rounded-lg',
                    'transition-colors duration-200',
                    'aspect-square',
                    active
                      ? 'bg-background-accent text-text-on-accent'
                      : 'bg-background-default hover:bg-background-medium'
                  )}
                >
                  {/* Main button area */}
                  <button
                    onClick={() => handleNavClick(item.path)}
                    className="flex-1 flex flex-col items-start justify-between p-5 no-drag text-left"
                  >
                    {/* Drag handle */}
                    <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity z-10">
                      <GripVertical className="w-4 h-4 text-text-muted" />
                    </div>

                    {/* Tag/Badge */}
                    {item.getTag && (
                      <div
                        className={cn(
                          'absolute top-3 px-2 py-1 rounded-full',
                          item.tagAlign === 'left' ? 'left-8' : 'right-8',
                          'bg-background-muted'
                        )}
                      >
                        <span className="text-xs font-mono text-text-muted">{item.getTag()}</span>
                      </div>
                    )}

                    {/* Icon and Label at bottom */}
                    <div className="mt-auto w-full">
                      <Icon className="w-6 h-6 mb-2" />
                      <h2 className="font-light text-left text-xl">{item.label}</h2>
                    </div>
                  </button>
                </motion.div>
              </motion.div>
            );
          })}

          {/* Spacer tiles to fill empty grid spaces - only render after grid is measured */}
          {!isOverlayMode &&
            gridMeasured &&
            gridColumns >= 2 &&
            Array.from({
              // For left/right: add extra rows of spacers to fill vertical space
              // For top/bottom: just fill remaining spaces in the last row
              length:
                navigationPosition === 'left' || navigationPosition === 'right'
                  ? ((gridColumns - (visibleItems.length % gridColumns)) % gridColumns) +
                    gridColumns * 6 // Fill last row + 6 more rows
                  : (gridColumns - (visibleItems.length % gridColumns)) % gridColumns,
            }).map((_, index) => (
              <div key={`spacer-${index}`} className="relative">
                <div className="w-full aspect-square rounded-lg bg-background-default" />
              </div>
            ))}
        </div>
      ) : null}
    </motion.div>
  );

  // Overlay mode: render with backdrop
  if (isOverlayMode) {
    return (
      <AnimatePresence>
        {isNavExpanded && (
          <div className="fixed inset-0 z-[10000]">
            {/* Backdrop */}
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              className="absolute inset-0 bg-black/20 backdrop-blur-sm"
              onClick={() => setIsNavExpanded(false)}
            />

            {/* Scrollable container for navigation panel */}
            <div className="absolute inset-0 overflow-y-auto pointer-events-none">
              <div className="min-h-full flex items-center justify-center p-8">
                <div className="pointer-events-auto max-w-3xl w-full">{navContent}</div>
              </div>
            </div>
          </div>
        )}
      </AnimatePresence>
    );
  }

  // Push mode: render inline
  if (!isNavExpanded) return null;
  return navContent;
};
