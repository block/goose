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
import type { Session } from '../../api';
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
  hasSubItems?: boolean;
}

interface RecentSession {
  id: string;
  name: string;
  created_at: string;
}

interface ActiveSession {
  sessionId: string;
  initialMessage?: string;
}

interface CondensedNavigationProps {
  className?: string;
  activeSessions?: ActiveSession[];
}

export const CondensedNavigation: React.FC<CondensedNavigationProps> = ({ className }) => {
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
    isCondensedIconOnly,
  } = useNavigationContext();

  const [draggedItem, setDraggedItem] = useState<string | null>(null);
  const [dragOverItem, setDragOverItem] = useState<string | null>(null);
  const [expandedItems, setExpandedItems] = useState<Set<string>>(new Set());
  const [chatPopoverOpen, setChatPopoverOpen] = useState(false);
  
  // Stats for tags
  const [todayChatsCount, setTodayChatsCount] = useState(0);
  const [totalSessions, setTotalSessions] = useState(0);
  const [recentSessions, setRecentSessions] = useState<RecentSession[]>([]);

  // Fetch stats when expanded
  useEffect(() => {
    if (isNavExpanded) {
      fetchNavigationData();
    }
  }, [isNavExpanded]);

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
        
        // Get the 10 most recent sessions
        const sorted = [...sessionsResponse.data.sessions]
          .sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())
          .slice(0, 10);
        setRecentSessions(sorted);
      }

      const insightsResponse = await getSessionInsights({ throwOnError: false });
      if (insightsResponse.data) {
        setTotalSessions(insightsResponse.data.totalSessions || 0);
      }
    } catch (error) {
      console.error('Failed to fetch navigation data:', error);
    }
  };

  // Build nav items with dynamic tags
  const getNavItems = (): NavItem[] => [
    { id: 'home', path: '/', label: 'Home', icon: Home },
    { 
      id: 'chat', 
      path: '/pair', 
      label: 'Chat', 
      icon: MessageSquare, 
      getTag: () => `${todayChatsCount}`,
      hasSubItems: true, // Always has sub-items for recent sessions
    },
    { id: 'history', path: '/sessions', label: 'History', icon: History, getTag: () => `${totalSessions}` },
    { id: 'recipes', path: '/recipes', label: 'Recipes', icon: FileText },
    { id: 'scheduler', path: '/schedules', label: 'Scheduler', icon: Clock },
    {
      id: 'extensions',
      path: '/extensions',
      label: 'Extensions',
      icon: Puzzle,
      getTag: () => {
        if (!extensionsList || !Array.isArray(extensionsList)) return '0/0';
        const enabled = extensionsList.filter(ext => ext.enabled).length;
        return `${enabled}/${extensionsList.length}`;
      },
    },
    { id: 'settings', path: '/settings', label: 'Settings', icon: Settings },
  ];

  const navItems = getNavItems();

  const getNavItemById = (id: string): NavItem | undefined => {
    return navItems.find(item => item.id === id);
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
  const currentSessionId = location.pathname === '/pair' ? searchParams.get('resumeSessionId') : null;

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

  const handleNavClick = useCallback((path: string) => {
    // For /pair, preserve the current session if one exists
    if (path === '/pair') {
      const sessionId = currentSessionId || lastSessionIdRef.current || chatContext?.chat?.sessionId;
      if (sessionId && sessionId.length > 0) {
        navigate(`/pair?resumeSessionId=${sessionId}`);
      } else {
        // No session - go to home to start a new chat
        navigate('/');
      }
    } else {
      navigate(path);
    }
    // Don't close nav on selection - only close via toggle button
  }, [navigate, currentSessionId, chatContext?.chat?.sessionId]);

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
    // Don't close nav on selection - only close via toggle button
  }, [setView]);

  const handleSessionClick = useCallback((sessionId: string) => {
    navigate(`/pair?resumeSessionId=${sessionId}`);
    // Don't close nav on selection - only close via toggle button
  }, [navigate]);

  const toggleExpanded = (itemId: string) => {
    setExpandedItems(prev => {
      const next = new Set(prev);
      if (next.has(itemId)) {
        next.delete(itemId);
      } else {
        next.add(itemId);
      }
      return next;
    });
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
    .filter(id => preferences.enabledItems.includes(id))
    .map(id => getNavItemById(id))
    .filter((item): item is NavItem => item !== undefined);

  const isOverlayMode = effectiveNavigationMode === 'overlay';

  // Truncate session message for display
  const truncateMessage = (msg?: string, maxLen = 20) => {
    if (!msg) return 'New Chat';
    return msg.length > maxLen ? msg.substring(0, maxLen) + '...' : msg;
  };

  const isTopPosition = navigationPosition === 'top';
  const isBottomPosition = navigationPosition === 'bottom';

  const navContent = (
    <motion.div
      initial={{ opacity: 0, scale: 0.95 }}
      animate={{ opacity: 1, scale: 1 }}
      exit={{ opacity: 0, scale: 0.95 }}
      transition={{ type: "spring", stiffness: 350, damping: 25 }}
      className={cn(
        'bg-app',
        isOverlayMode && 'rounded-xl backdrop-blur-md shadow-lg p-2',
        isVertical ? 'flex flex-col gap-[2px] h-full' : 'flex flex-row items-stretch gap-[2px]',
        // Add 2px padding on the edge facing the content for vertical
        !isOverlayMode && navigationPosition === 'left' && 'pr-[2px]',
        !isOverlayMode && navigationPosition === 'right' && 'pl-[2px]',
        // Add 2px padding on the edge facing the content for horizontal
        !isOverlayMode && isTopPosition && 'pb-[2px] pt-0',
        !isOverlayMode && isBottomPosition && 'pt-[2px] pb-0',
        className
      )}
    >
      {/* Top spacer (vertical only) */}
      {isVertical && (
        <div className={cn(
          "bg-background-default rounded-lg w-full flex-shrink-0",
          isCondensedIconOnly ? "h-[80px]" : "h-[40px]"
        )} />
      )}

      {/* Left spacer (horizontal top position only) */}
      {!isVertical && isTopPosition && (
        <div className="bg-background-default rounded-lg self-stretch w-[160px] flex-shrink-0" />
      )}

      {/* Navigation items */}
      {visibleItems.map((item, index) => {
        const Icon = item.icon;
        const active = isActive(item.path);
        const isDragging = draggedItem === item.id;
        const isDragOver = dragOverItem === item.id;
        const isChatItem = item.id === 'chat';
        const isItemExpanded = expandedItems.has(item.id);

        return (
          <motion.div
            key={item.id}
            draggable
            onDragStart={(e) => handleDragStart(e as unknown as React.DragEvent, item.id)}
            onDragOver={(e) => handleDragOver(e as unknown as React.DragEvent, item.id)}
            onDrop={(e) => handleDrop(e as unknown as React.DragEvent, item.id)}
            onDragEnd={handleDragEnd}
            initial={{ opacity: 0, [isVertical ? 'x' : 'y']: 20, scale: 0.9 }}
            animate={{ 
              opacity: isDragging ? 0.5 : 1, 
              [isVertical ? 'x' : 'y']: 0, 
              scale: isDragging ? 0.95 : 1,
            }}
            transition={{
              type: "spring",
              stiffness: 350,
              damping: 25,
              delay: index * 0.02,
            }}
            className={cn(
              'relative cursor-move group',
              isVertical ? 'w-full' : 'flex-shrink-0',
              isDragOver && 'ring-2 ring-blue-500 rounded-lg'
            )}
          >
            <div className={cn(
              'flex flex-col',
              isVertical ? 'w-full' : ''
            )}>
              {/* Chat item with dropdown in horizontal mode OR icon-only mode */}
              {isChatItem && (!isVertical || isCondensedIconOnly) ? (
                <DropdownMenu open={chatPopoverOpen} onOpenChange={setChatPopoverOpen}>
                  <DropdownMenuTrigger asChild>
                    <motion.button
                      whileHover={{ scale: 1.02 }}
                      whileTap={{ scale: 0.98 }}
                      className={cn(
                        'flex flex-row items-center justify-center gap-2',
                        'relative rounded-lg transition-colors duration-200 no-drag',
                        isCondensedIconOnly ? 'p-2.5' : 'px-3 py-2.5',
                        active
                          ? 'bg-background-accent text-text-on-accent'
                          : 'bg-background-default hover:bg-background-medium'
                      )}
                    >
                      <Icon className="w-5 h-5 flex-shrink-0" />
                      {!isCondensedIconOnly && (
                        <>
                          <span className="text-sm font-medium text-left hidden min-[1200px]:block">
                            {item.label}
                          </span>
                          {item.getTag && (
                            <div className="flex items-center gap-1 flex-shrink-0 hidden min-[1200px]:flex">
                              <span className={cn(
                                'text-xs font-mono px-2 py-0.5 rounded-full',
                                active
                                  ? 'bg-background-default/20 text-text-on-accent/80'
                                  : 'bg-background-muted text-text-muted'
                              )}>
                                {item.getTag()}
                              </span>
                            </div>
                          )}
                        </>
                      )}
                    </motion.button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent 
                    className="w-64 p-1 bg-background-default border-border-subtle rounded-lg shadow-lg"
                    side={isCondensedIconOnly ? (navigationPosition === 'left' ? 'right' : 'left') : (isTopPosition ? 'bottom' : 'top')}
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
                    
                    {recentSessions.length > 0 && (
                      <DropdownMenuSeparator className="my-1" />
                    )}
                    
                    {/* Recent sessions */}
                    {recentSessions.map((session) => (
                      <DropdownMenuItem
                        key={session.id}
                        onClick={() => handleSessionClick(session.id)}
                        className="flex items-center gap-2 px-3 py-2 text-sm rounded-lg cursor-pointer"
                      >
                        <MessageSquare className="w-4 h-4 flex-shrink-0 text-text-muted" />
                        <span className="truncate">
                          {truncateMessage(session.name, 30)}
                        </span>
                      </DropdownMenuItem>
                    ))}
                    
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
                </DropdownMenu>
              ) : (
                /* Regular button for non-chat items or vertical mode (not icon-only) */
                <motion.button
                  onClick={() => {
                    if (isChatItem && isVertical && !isCondensedIconOnly) {
                      toggleExpanded(item.id);
                    } else {
                      handleNavClick(item.path);
                    }
                  }}
                  whileHover={{ scale: 1.02 }}
                  whileTap={{ scale: 0.98 }}
                  className={cn(
                    'flex flex-row items-center gap-2',
                    'relative rounded-lg transition-colors duration-200 no-drag',
                    isCondensedIconOnly 
                      ? 'justify-center p-2.5' 
                      : isVertical 
                        ? 'w-full pl-2 pr-4 py-2.5' 
                        : 'px-3 py-2.5',
                    active
                      ? 'bg-background-accent text-text-on-accent'
                      : 'bg-background-default hover:bg-background-medium'
                  )}
                >
                  {/* Drag handle - visible on hover (not in icon-only mode) */}
                  {!isCondensedIconOnly && (
                    <div className={cn(
                      'opacity-0 group-hover:opacity-100 transition-opacity flex-shrink-0',
                      !isVertical && 'hidden'
                    )}>
                      <GripVertical className="w-4 h-4 text-text-muted" />
                    </div>
                  )}

                  {/* Icon */}
                  <Icon className="w-5 h-5 flex-shrink-0" />
                  
                  {/* Label - hidden in icon-only mode and on horizontal unless wide screen */}
                  {!isCondensedIconOnly && (
                    <span className={cn(
                      'text-sm font-medium text-left',
                      isVertical ? 'flex-1' : 'hidden min-[1200px]:block'
                    )}>
                      {item.label}
                    </span>
                  )}

                  {/* Tag/Badge - hidden in icon-only mode */}
                  {!isCondensedIconOnly && item.getTag && (
                    <div className={cn(
                      'flex items-center gap-1 flex-shrink-0',
                      !isVertical && 'hidden min-[1200px]:flex'
                    )}>
                      <span className={cn(
                        'text-xs font-mono px-2 py-0.5 rounded-full',
                        active
                          ? 'bg-background-default/20 text-text-on-accent/80'
                          : 'bg-background-muted text-text-muted'
                      )}>
                        {item.getTag()}
                      </span>
                    </div>
                  )}

                  {/* Expand indicator for chat item (vertical only, not icon-only) - after count */}
                  {!isCondensedIconOnly && isChatItem && isVertical && (
                    <div className="flex-shrink-0">
                      {isItemExpanded ? (
                        <ChevronDown className="w-3 h-3 text-text-muted" />
                      ) : (
                        <ChevronRight className="w-3 h-3 text-text-muted" />
                      )}
                    </div>
                  )}
                </motion.button>
              )}

              {/* Recent sessions dropdown (vertical only, not icon-only mode) */}
              <AnimatePresence>
                {isChatItem && isItemExpanded && isVertical && !isCondensedIconOnly && (
                  <motion.div
                    initial={{ height: 0, opacity: 0 }}
                    animate={{ height: 'auto', opacity: 1 }}
                    exit={{ height: 0, opacity: 0 }}
                    transition={{ duration: 0.2 }}
                    className="overflow-hidden"
                  >
                    <div className="py-[2px] flex flex-col gap-[2px]">
                      {/* New chat button at top */}
                      <button
                        onClick={handleNewChat}
                        className={cn(
                          'w-full text-left pl-2 pr-4 py-1.5 text-xs rounded-lg',
                          'bg-background-default hover:bg-background-medium transition-colors',
                          'flex items-center gap-2 text-text-default font-medium'
                        )}
                      >
                        {/* Spacer to align with parent icon (drag handle width + gap) */}
                        <span className="w-4 flex-shrink-0" />
                        <span className="w-3 h-3 flex-shrink-0 flex items-center justify-center">+</span>
                        <span>New Chat</span>
                      </button>
                      
                      {/* Recent sessions */}
                      {recentSessions.map((session) => (
                        <button
                          key={session.id}
                          onClick={() => handleSessionClick(session.id)}
                          className={cn(
                            'w-full text-left pl-2 pr-4 py-1.5 text-xs rounded-lg',
                            'bg-background-default hover:bg-background-medium transition-colors',
                            'flex items-center gap-2'
                          )}
                        >
                          {/* Spacer to align with parent icon (drag handle width + gap) */}
                          <span className="w-4 flex-shrink-0" />
                          <MessageSquare className="w-3 h-3 flex-shrink-0 text-text-muted" />
                          <span className="truncate text-text-default">
                            {truncateMessage(session.name)}
                          </span>
                        </button>
                      ))}
                      
                      {/* Show All button */}
                      {totalSessions > 10 && (
                        <button
                          onClick={() => handleNavClick('/sessions')}
                          className={cn(
                            'w-full text-left pl-2 pr-4 py-1.5 text-xs rounded-lg',
                            'bg-background-default hover:bg-background-medium transition-colors',
                            'flex items-center gap-2 text-text-muted'
                          )}
                        >
                          {/* Spacer to align with parent icon (drag handle width + gap) */}
                          <span className="w-4 flex-shrink-0" />
                          <History className="w-3 h-3 flex-shrink-0" />
                          <span>Show All ({totalSessions})</span>
                        </button>
                      )}
                    </div>
                  </motion.div>
                )}
              </AnimatePresence>
            </div>
          </motion.div>
        );
      })}

      {/* Spacer to extend to bottom (vertical only - both regular and icon-only modes) */}
      {isVertical && (
        <div className="bg-background-default rounded-lg flex-1 w-full min-h-[20px]" />
      )}

      {/* Right spacer (horizontal - both top and bottom positions) - full width to fill remaining space */}
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
            
            {/* Positioned navigation panel */}
            <div className={cn(
              'absolute p-4 pointer-events-none',
              navigationPosition === 'top' && 'top-0 left-1/2 -translate-x-1/2 pt-16',
              navigationPosition === 'bottom' && 'bottom-0 left-1/2 -translate-x-1/2 pb-8',
              navigationPosition === 'left' && 'left-0 top-1/2 -translate-y-1/2 pl-4',
              navigationPosition === 'right' && 'right-0 top-1/2 -translate-y-1/2 pr-4'
            )}>
              <div className="pointer-events-auto">
                {navContent}
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
