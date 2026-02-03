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
  Menu,
  ChevronDown,
  ChevronRight,
  Plus,
  ChefHat,
} from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';
import { useNavigationContext } from './NavigationContext';
import { cn } from '../../utils';
import { listSessions } from '../../api';
import { useChatContext } from '../../contexts/ChatContext';
import { useNavigation } from '../../hooks/useNavigation';
import { startNewSession, resumeSession, shouldShowNewChatTitle } from '../../sessions';
import { getInitialWorkingDir } from '../../utils/workingDir';
import { useSidebarSessionStatus } from '../../hooks/useSidebarSessionStatus';
import { SessionIndicators } from '../SessionIndicators';
import { AppEvents } from '../../constants/events';
import type { Session } from '../../api';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '../ui/dropdown-menu';
import * as PopoverPrimitive from '@radix-ui/react-popover';

interface NavItem {
  id: string;
  path: string;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  getTag?: () => string;
  hasSubItems?: boolean;
}

interface CondensedNavigationProps {
  className?: string;
}

export const CondensedNavigation: React.FC<CondensedNavigationProps> = ({ className }) => {
  const navigate = useNavigate();
  const location = useLocation();
  const {
    isNavExpanded,
    setIsNavExpanded,
    effectiveNavigationMode,
    navigationPosition,
    preferences,
    updatePreferences,
    isCondensedIconOnly,
    isChatExpanded,
    setIsChatExpanded,
  } = useNavigationContext();

  const [draggedItem, setDraggedItem] = useState<string | null>(null);
  const [dragOverItem, setDragOverItem] = useState<string | null>(null);
  const [chatPopoverOpen, setChatPopoverOpen] = useState(false);
  const [newChatHoverOpen, setNewChatHoverOpen] = useState(false);
  const hoverTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const [recentSessions, setRecentSessions] = useState<Session[]>([]);

  // Ref for focusing navigation when opened
  const navContainerRef = useRef<HTMLDivElement>(null);

  // Hover handlers with delay to allow mouse to travel between trigger and popover
  const handleHoverOpen = useCallback(() => {
    if (hoverTimeoutRef.current) {
      clearTimeout(hoverTimeoutRef.current);
      hoverTimeoutRef.current = null;
    }
    setNewChatHoverOpen(true);
  }, []);

  const handleHoverClose = useCallback(() => {
    hoverTimeoutRef.current = setTimeout(() => {
      setNewChatHoverOpen(false);
    }, 300);
  }, []);

  // Cleanup timeout on unmount
  useEffect(() => {
    return () => {
      if (hoverTimeoutRef.current) {
        clearTimeout(hoverTimeoutRef.current);
      }
    };
  }, []);

  // Track session for /pair navigation
  const [searchParams] = useSearchParams();
  const activeSessionId = searchParams.get('resumeSessionId') ?? undefined;

  // Use sidebar session status hook for streaming/unread indicators
  const { getSessionStatus, clearUnread } = useSidebarSessionStatus(activeSessionId);

  // Fetch sessions when expanded and focus navigation
  useEffect(() => {
    if (isNavExpanded) {
      fetchNavigationData();
      // Focus the navigation container for keyboard navigation
      // Use a small delay to ensure the element is rendered
      requestAnimationFrame(() => {
        navContainerRef.current?.focus();
      });
    }
  }, [isNavExpanded]);

  const fetchNavigationData = async () => {
    try {
      const sessionsResponse = await listSessions({ throwOnError: false });
      if (sessionsResponse.data) {
        // Get the 5 most recent sessions
        const sorted = [...sessionsResponse.data.sessions]
          .sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())
          .slice(0, 5);
        setRecentSessions(sorted);
      }
    } catch (error) {
      console.error('Failed to fetch navigation data:', error);
    }
  };

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
          return [session, ...prev].slice(0, 5);
        });
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
            const apiSessions = response.data.sessions.slice(0, 5);
            setRecentSessions((prev) => {
              // Merge: keep empty local sessions not in API, add API sessions
              const emptyLocalSessions = prev.filter(
                (local) =>
                  local.message_count === 0 && !apiSessions.some((api) => api.id === local.id)
              );
              return [...emptyLocalSessions, ...apiSessions].slice(0, 5);
            });
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

  // Build nav items
  const getNavItems = (): NavItem[] => [
    { id: 'home', path: '/', label: 'Home', icon: Home },
    {
      id: 'chat',
      path: '/pair',
      label: 'Chat',
      icon: MessageSquare,
      hasSubItems: true, // Always has sub-items for recent sessions
    },
    {
      id: 'history',
      path: '/sessions',
      label: 'History',
      icon: History,
    },
    { id: 'recipes', path: '/recipes', label: 'Recipes', icon: FileText },
    { id: 'scheduler', path: '/schedules', label: 'Scheduler', icon: Clock },
    {
      id: 'extensions',
      path: '/extensions',
      label: 'Extensions',
      icon: Puzzle,
    },
    { id: 'settings', path: '/settings', label: 'Settings', icon: Settings },
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

  const chatContext = useChatContext();
  const lastSessionIdRef = useRef<string | null>(null);
  const currentSessionId =
    location.pathname === '/pair' ? searchParams.get('resumeSessionId') : null;

  // Keep track of last session ID
  useEffect(() => {
    if (currentSessionId) {
      lastSessionIdRef.current = currentSessionId;
    }
  }, [currentSessionId]);

  // Use setView for navigation to pair view (matches original AppSidebar behavior)
  const setView = useNavigation();

  // Ref to track recent sessions without causing re-renders
  const recentSessionsRef = useRef<Session[]>([]);
  useEffect(() => {
    recentSessionsRef.current = recentSessions as Session[];
  }, [recentSessions]);

  // Guard ref to prevent duplicate session creation
  const isCreatingSessionRef = useRef(false);

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
    const emptyNewSession = recentSessionsRef.current.find((s) => shouldShowNewChatTitle(s));

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

  const toggleChatExpanded = () => {
    setIsChatExpanded(!isChatExpanded);
  };

  const isActive = (path: string) => location.pathname === path;
  const isVertical = navigationPosition === 'left' || navigationPosition === 'right';

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

  // Get display name for session - prioritize recipe title, then session name
  const getSessionDisplayName = (session: Session): string => {
    if (session.recipe?.title) {
      return session.recipe.title;
    }
    if (shouldShowNewChatTitle(session)) {
      return 'New Chat';
    }
    return session.name;
  };

  // Truncate session message for display
  const truncateMessage = (msg?: string, maxLen = 20) => {
    if (!msg) return 'New Chat';
    return msg.length > maxLen ? msg.substring(0, maxLen) + '...' : msg;
  };

  const isTopPosition = navigationPosition === 'top';
  const isBottomPosition = navigationPosition === 'bottom';

  const navContent = (
    <motion.div
      ref={navContainerRef}
      tabIndex={-1}
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={{ duration: 0.15 }}
      className={cn(
        'bg-app outline-none',
        isOverlayMode && 'rounded-xl backdrop-blur-md shadow-lg p-2',
        isVertical ? 'flex flex-col gap-[2px] h-full' : 'flex flex-row items-stretch gap-[2px]',
        // Add 2px padding on the edge facing the content for vertical (only when not icon-only)
        !isOverlayMode && navigationPosition === 'left' && !isCondensedIconOnly && 'pr-[2px]',
        !isOverlayMode && navigationPosition === 'right' && !isCondensedIconOnly && 'pl-[2px]',
        // Add 2px padding on the edge facing the content for horizontal
        !isOverlayMode && isTopPosition && 'pb-[2px] pt-0',
        !isOverlayMode && isBottomPosition && 'pt-[2px] pb-0',
        // Allow hover buttons to overflow outside the nav container
        !isCondensedIconOnly && 'overflow-visible',
        className
      )}
    >
      {/* Top spacer (vertical only) */}
      {isVertical && (
        <div
          className={cn(
            'bg-background-default rounded-lg flex-shrink-0',
            isCondensedIconOnly ? 'h-[80px] w-[40px]' : 'h-[48px] w-full'
          )}
        />
      )}

      {/* Left spacer (horizontal top position only) */}
      {!isVertical && isTopPosition && (
        <div className="bg-background-default rounded-lg self-stretch w-[160px] flex-shrink-0" />
      )}

      {/* Navigation items container (vertical only) */}
      {isVertical ? (
        <div className="flex-1 min-h-0 flex flex-col gap-[2px]">
          {visibleItems.map((item, index) => {
            const Icon = item.icon;
            const active = isActive(item.path);
            const isDragging = draggedItem === item.id;
            const isDragOver = dragOverItem === item.id;
            const isChatItem = item.id === 'chat';

            return (
              <motion.div
                key={item.id}
                draggable
                onDragStart={(e) => handleDragStart(e as unknown as React.DragEvent, item.id)}
                onDragOver={(e) => handleDragOver(e as unknown as React.DragEvent, item.id)}
                onDrop={(e) => handleDrop(e as unknown as React.DragEvent, item.id)}
                onDragEnd={handleDragEnd}
                initial={{ opacity: 0 }}
                animate={{
                  opacity: isDragging ? 0.5 : 1,
                }}
                transition={{
                  duration: 0.15,
                  delay: index * 0.02,
                }}
                className={cn(
                  'relative cursor-move group',
                  isCondensedIconOnly ? 'flex-shrink-0' : 'w-full flex-shrink-0',
                  isDragOver && 'ring-2 ring-blue-500 rounded-lg',
                  isChatItem && !isCondensedIconOnly && 'overflow-visible'
                )}
              >
                <div
                  className={cn(
                    'flex flex-col',
                    isCondensedIconOnly ? 'items-start' : 'w-full',
                    isChatItem && !isCondensedIconOnly && 'overflow-visible'
                  )}
                >
                  {/* Chat item with dropdown in icon-only mode */}
                  {isChatItem && isCondensedIconOnly ? (
                    <DropdownMenu open={chatPopoverOpen} onOpenChange={setChatPopoverOpen}>
                      <DropdownMenuTrigger asChild>
                        <button
                          className={cn(
                            'flex items-center justify-center',
                            'rounded-lg transition-colors duration-200 no-drag',
                            'p-2.5',
                            active
                              ? 'bg-background-accent text-text-on-accent'
                              : 'bg-background-default hover:bg-background-medium'
                          )}
                        >
                          <Icon className="w-5 h-5" />
                        </button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent
                        className="w-64 p-1 bg-background-default border-border-subtle rounded-lg shadow-lg"
                        side={navigationPosition === 'left' ? 'right' : 'left'}
                        align="start"
                        sideOffset={8}
                      >
                        <DropdownMenuItem
                          onClick={handleNewChat}
                          className="flex items-center gap-2 px-3 py-2 text-sm rounded-lg cursor-pointer"
                        >
                          <Plus className="w-4 h-4 flex-shrink-0" />
                          <span>New Chat</span>
                        </DropdownMenuItem>
                        {recentSessions.length > 0 && <DropdownMenuSeparator className="my-1" />}
                        {recentSessions.map((session) => {
                          const status = getSessionStatus(session.id);
                          const isStreaming = status?.streamState === 'streaming';
                          const hasError = status?.streamState === 'error';
                          const hasUnread = status?.hasUnreadActivity ?? false;
                          const isActiveSession = session.id === activeSessionId;
                          return (
                            <DropdownMenuItem
                              key={session.id}
                              onClick={() => {
                                clearUnread(session.id);
                                handleSessionClick(session.id);
                              }}
                              className={cn(
                                'flex items-center gap-2 px-3 py-2 text-sm rounded-lg cursor-pointer',
                                isActiveSession && 'bg-background-medium'
                              )}
                            >
                              {session.recipe ? (
                                <ChefHat className="w-4 h-4 flex-shrink-0 text-text-muted" />
                              ) : (
                                <MessageSquare className="w-4 h-4 flex-shrink-0 text-text-muted" />
                              )}
                              <span className="truncate flex-1">
                                {truncateMessage(getSessionDisplayName(session), 30)}
                              </span>
                              <SessionIndicators
                                isStreaming={isStreaming}
                                hasUnread={hasUnread}
                                hasError={hasError}
                              />
                            </DropdownMenuItem>
                          );
                        })}
                        {recentSessions.length > 0 && (
                          <>
                            <DropdownMenuSeparator className="my-1" />
                            <DropdownMenuItem
                              onClick={() => handleNavClick('/sessions')}
                              className="flex items-center gap-2 px-3 py-2 text-sm rounded-lg cursor-pointer text-text-muted"
                            >
                              <History className="w-4 h-4 flex-shrink-0" />
                              <span>Show All</span>
                            </DropdownMenuItem>
                          </>
                        )}
                      </DropdownMenuContent>
                    </DropdownMenu>
                  ) : (
                    <>
                      {/* Chat row with hover popover for new chat button */}
                      {isChatItem && !isCondensedIconOnly ? (
                        <PopoverPrimitive.Root open={newChatHoverOpen}>
                          <PopoverPrimitive.Trigger asChild>
                            <motion.button
                              onClick={toggleChatExpanded}
                              onMouseEnter={handleHoverOpen}
                              onMouseLeave={handleHoverClose}
                              whileHover={{ scale: 1.02 }}
                              whileTap={{ scale: 0.98 }}
                              className={cn(
                                'flex flex-row items-center gap-2 outline-none',
                                'relative rounded-lg transition-colors duration-200 no-drag',
                                'w-full pl-2 pr-4 py-2.5',
                                active
                                  ? 'bg-background-accent text-text-on-accent'
                                  : 'bg-background-default hover:bg-background-medium'
                              )}
                            >
                              <div className="opacity-0 group-hover:opacity-100 transition-opacity flex-shrink-0">
                                <GripVertical className="w-4 h-4 text-text-muted" />
                              </div>
                              <Icon className="w-5 h-5 flex-shrink-0" />
                              <span className="text-sm font-medium text-left flex-1">
                                {item.label}
                              </span>
                              <div className="flex-shrink-0">
                                {isChatExpanded ? (
                                  <ChevronDown className="w-3 h-3 text-text-muted" />
                                ) : (
                                  <ChevronRight className="w-3 h-3 text-text-muted" />
                                )}
                              </div>
                            </motion.button>
                          </PopoverPrimitive.Trigger>
                          <PopoverPrimitive.Portal>
                            <PopoverPrimitive.Content
                              side={navigationPosition === 'left' ? 'right' : 'left'}
                              align="center"
                              sideOffset={4}
                              onMouseEnter={handleHoverOpen}
                              onMouseLeave={handleHoverClose}
                              className="z-[9999] outline-none"
                            >
                              <button
                                onClick={(e) => {
                                  e.stopPropagation();
                                  handleNewChat();
                                  setNewChatHoverOpen(false);
                                }}
                                className={cn(
                                  'p-1.5 rounded-md outline-none',
                                  'bg-background-medium hover:bg-background-accent hover:text-text-on-accent',
                                  'flex items-center justify-center',
                                  'shadow-sm transition-all duration-150',
                                  'hover:scale-110 active:scale-95'
                                )}
                                title="New Chat"
                              >
                                <Plus className="w-4 h-4" />
                              </button>
                            </PopoverPrimitive.Content>
                          </PopoverPrimitive.Portal>
                        </PopoverPrimitive.Root>
                      ) : (
                        <motion.button
                          onClick={() => handleNavClick(item.path)}
                          whileHover={{ scale: 1.02 }}
                          whileTap={{ scale: 0.98 }}
                          className={cn(
                            'flex flex-row items-center gap-2',
                            'relative rounded-lg transition-colors duration-200 no-drag',
                            isCondensedIconOnly
                              ? 'justify-center p-2.5'
                              : 'w-full pl-2 pr-4 py-2.5',
                            active
                              ? 'bg-background-accent text-text-on-accent'
                              : 'bg-background-default hover:bg-background-medium'
                          )}
                        >
                          {!isCondensedIconOnly && (
                            <div className="opacity-0 group-hover:opacity-100 transition-opacity flex-shrink-0">
                              <GripVertical className="w-4 h-4 text-text-muted" />
                            </div>
                          )}
                          <Icon className="w-5 h-5 flex-shrink-0" />
                          {!isCondensedIconOnly && (
                            <span className="text-sm font-medium text-left flex-1">
                              {item.label}
                            </span>
                          )}
                          {!isCondensedIconOnly && item.getTag && (
                            <div className="flex items-center gap-1 flex-shrink-0">
                              <span
                                className={cn(
                                  'text-xs font-mono px-2 py-0.5 rounded-full',
                                  active
                                    ? 'bg-background-default/20 text-text-on-accent/80'
                                    : 'bg-background-muted text-text-muted'
                                )}
                              >
                                {item.getTag()}
                              </span>
                            </div>
                          )}
                        </motion.button>
                      )}
                    </>
                  )}
                  <AnimatePresence>
                    {isChatItem && isChatExpanded && !isCondensedIconOnly && (
                      <motion.div
                        initial={{ height: 0, opacity: 0 }}
                        animate={{ height: 'auto', opacity: 1 }}
                        exit={{ height: 0, opacity: 0 }}
                        transition={{ duration: 0.2 }}
                        className="overflow-hidden mt-[2px]"
                      >
                        <div className="bg-background-default rounded-lg py-1 flex flex-col gap-[2px]">
                          {recentSessions.map((session) => {
                            const status = getSessionStatus(session.id);
                            const isStreaming = status?.streamState === 'streaming';
                            const hasError = status?.streamState === 'error';
                            const hasUnread = status?.hasUnreadActivity ?? false;
                            const isActiveSession = session.id === activeSessionId;
                            return (
                              <button
                                key={session.id}
                                onClick={() => {
                                  clearUnread(session.id);
                                  handleSessionClick(session.id);
                                }}
                                className={cn(
                                  'w-full text-left py-1.5 px-2 text-xs rounded-md',
                                  'hover:bg-background-medium transition-colors',
                                  'flex items-center gap-2',
                                  isActiveSession && 'bg-background-medium'
                                )}
                              >
                                <div className="w-4 flex-shrink-0" />{' '}
                                {/* Spacer to align with grip icon */}
                                {session.recipe ? (
                                  <ChefHat className="w-4 h-4 flex-shrink-0 text-text-muted" />
                                ) : (
                                  <MessageSquare className="w-4 h-4 flex-shrink-0 text-text-muted" />
                                )}
                                <span className="truncate text-text-default flex-1">
                                  {truncateMessage(getSessionDisplayName(session))}
                                </span>
                                <SessionIndicators
                                  isStreaming={isStreaming}
                                  hasUnread={hasUnread}
                                  hasError={hasError}
                                />
                              </button>
                            );
                          })}
                        </div>
                      </motion.div>
                    )}
                  </AnimatePresence>
                </div>
              </motion.div>
            );
          })}

          {/* Bottom filler block - fills remaining space below nav items */}
          <div
            className={cn(
              'bg-background-default rounded-lg flex-1 min-h-[40px]',
              isCondensedIconOnly ? 'w-[40px]' : 'w-full'
            )}
          />
        </div>
      ) : (
        /* Horizontal navigation items */
        visibleItems.map((item, index) => {
          const Icon = item.icon;
          const active = isActive(item.path);
          const isDragging = draggedItem === item.id;
          const isDragOver = dragOverItem === item.id;
          const isChatItem = item.id === 'chat';

          return (
            <motion.div
              key={item.id}
              draggable
              onDragStart={(e) => handleDragStart(e as unknown as React.DragEvent, item.id)}
              onDragOver={(e) => handleDragOver(e as unknown as React.DragEvent, item.id)}
              onDrop={(e) => handleDrop(e as unknown as React.DragEvent, item.id)}
              onDragEnd={handleDragEnd}
              initial={{ opacity: 0 }}
              animate={{
                opacity: isDragging ? 0.5 : 1,
              }}
              transition={{
                duration: 0.15,
                delay: index * 0.02,
              }}
              className={cn(
                'relative cursor-move group flex-shrink-0',
                isDragOver && 'ring-2 ring-blue-500 rounded-lg',
                isChatItem && !isCondensedIconOnly && 'overflow-visible'
              )}
            >
              <div className="flex flex-col">
                {/* Chat item with dropdown in horizontal mode */}
                {isChatItem ? (
                  <>
                    <DropdownMenu open={chatPopoverOpen} onOpenChange={setChatPopoverOpen}>
                      <DropdownMenuTrigger asChild>
                        <motion.button
                          whileHover={{ scale: 1.02 }}
                          whileTap={{ scale: 0.98 }}
                          className={cn(
                            'flex flex-row items-center justify-center gap-2',
                            'relative rounded-lg transition-colors duration-200 no-drag',
                            'px-3 py-2.5',
                            active
                              ? 'bg-background-accent text-text-on-accent'
                              : 'bg-background-default hover:bg-background-medium'
                          )}
                        >
                          <Icon className="w-5 h-5 flex-shrink-0" />
                          <span className="text-sm font-medium text-left hidden min-[1200px]:block">
                            {item.label}
                          </span>
                        </motion.button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent
                        className="w-64 p-1 bg-background-default border-border-subtle rounded-lg shadow-lg"
                        side={isTopPosition ? 'bottom' : 'top'}
                        align="start"
                        sideOffset={8}
                      >
                        <DropdownMenuItem
                          onClick={handleNewChat}
                          className="flex items-center gap-2 px-3 py-2 text-sm rounded-lg cursor-pointer"
                        >
                          <Plus className="w-4 h-4 flex-shrink-0" />
                          <span>New Chat</span>
                        </DropdownMenuItem>
                        {recentSessions.length > 0 && <DropdownMenuSeparator className="my-1" />}
                        {recentSessions.map((session) => {
                          const status = getSessionStatus(session.id);
                          const isStreaming = status?.streamState === 'streaming';
                          const hasError = status?.streamState === 'error';
                          const hasUnread = status?.hasUnreadActivity ?? false;
                          const isActiveSession = session.id === activeSessionId;
                          return (
                            <DropdownMenuItem
                              key={session.id}
                              onClick={() => {
                                clearUnread(session.id);
                                handleSessionClick(session.id);
                              }}
                              className={cn(
                                'flex items-center gap-2 px-3 py-2 text-sm rounded-lg cursor-pointer',
                                isActiveSession && 'bg-background-medium'
                              )}
                            >
                              {session.recipe ? (
                                <ChefHat className="w-4 h-4 flex-shrink-0 text-text-muted" />
                              ) : (
                                <MessageSquare className="w-4 h-4 flex-shrink-0 text-text-muted" />
                              )}
                              <span className="truncate flex-1">
                                {truncateMessage(getSessionDisplayName(session), 30)}
                              </span>
                              <SessionIndicators
                                isStreaming={isStreaming}
                                hasUnread={hasUnread}
                                hasError={hasError}
                              />
                            </DropdownMenuItem>
                          );
                        })}
                        {recentSessions.length > 0 && (
                          <>
                            <DropdownMenuSeparator className="my-1" />
                            <DropdownMenuItem
                              onClick={() => handleNavClick('/sessions')}
                              className="flex items-center gap-2 px-3 py-2 text-sm rounded-lg cursor-pointer text-text-muted"
                            >
                              <History className="w-4 h-4 flex-shrink-0" />
                              <span>Show All</span>
                            </DropdownMenuItem>
                          </>
                        )}
                      </DropdownMenuContent>
                    </DropdownMenu>
                    {!chatPopoverOpen && (
                      <motion.button
                        onClick={(e) => {
                          e.stopPropagation();
                          handleNewChat();
                        }}
                        whileHover={{ scale: 1.1 }}
                        whileTap={{ scale: 0.95 }}
                        className={cn(
                          'absolute left-1/2 -translate-x-1/2 p-1.5 rounded-md z-10',
                          'opacity-0 group-hover:opacity-100 transition-opacity',
                          'bg-background-medium hover:bg-background-accent hover:text-text-on-accent',
                          'flex items-center justify-center',
                          isTopPosition ? '-bottom-9' : '-top-9'
                        )}
                        title="New Chat"
                      >
                        <Plus className="w-4 h-4" />
                      </motion.button>
                    )}
                  </>
                ) : (
                  <motion.button
                    onClick={() => handleNavClick(item.path)}
                    whileHover={{ scale: 1.02 }}
                    whileTap={{ scale: 0.98 }}
                    className={cn(
                      'flex flex-row items-center gap-2 px-3 py-2.5',
                      'relative rounded-lg transition-colors duration-200 no-drag',
                      active
                        ? 'bg-background-accent text-text-on-accent'
                        : 'bg-background-default hover:bg-background-medium'
                    )}
                  >
                    <Icon className="w-5 h-5 flex-shrink-0" />
                    <span className="text-sm font-medium text-left hidden min-[1200px]:block">
                      {item.label}
                    </span>
                  </motion.button>
                )}
              </div>
            </motion.div>
          );
        })
      )}

      {/* Right spacer (horizontal only) */}
      {!isVertical && (
        <div className="bg-background-default rounded-lg self-stretch flex-1 min-w-[40px]" />
      )}
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
              <div
                className={cn(
                  'min-h-full flex p-4',
                  navigationPosition === 'top' && 'items-start justify-center pt-16',
                  navigationPosition === 'bottom' && 'items-end justify-center pb-8',
                  navigationPosition === 'left' && 'items-center justify-start pl-4',
                  navigationPosition === 'right' && 'items-center justify-end pr-4'
                )}
              >
                <div className="pointer-events-auto">{navContent}</div>
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

// Trigger button to open navigation
interface NavTriggerProps {
  className?: string;
}

export const NavTrigger: React.FC<NavTriggerProps> = ({ className }) => {
  const { isNavExpanded, setIsNavExpanded } = useNavigationContext();

  return (
    <button
      onClick={() => setIsNavExpanded(!isNavExpanded)}
      className={cn(
        'p-2 rounded-lg transition-all duration-150',
        'hover:bg-background-medium',
        'flex items-center justify-center',
        isNavExpanded && 'bg-background-medium',
        className
      )}
      aria-label={isNavExpanded ? 'Close navigation' : 'Open navigation'}
    >
      <Menu className="w-5 h-5 text-text-muted" />
    </button>
  );
};
