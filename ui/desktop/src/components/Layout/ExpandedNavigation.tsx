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
  ChevronDown,
  ChevronRight,
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
  initialMessage?: string;
}

interface ExpandedNavigationProps {
  className?: string;
  activeSessions?: ActiveSession[];
}

export const ExpandedNavigation: React.FC<ExpandedNavigationProps> = ({ className, activeSessions = [] }) => {
  const navigate = useNavigate();
  const location = useLocation();
  const { extensionsList } = useConfig();
  const {
    isNavExpanded,
    setIsNavExpanded,
    navigationMode,
    preferences,
    updatePreferences,
  } = useNavigationContext();

  const [draggedItem, setDraggedItem] = useState<string | null>(null);
  const [dragOverItem, setDragOverItem] = useState<string | null>(null);
  const [expandedItems, setExpandedItems] = useState<Set<string>>(new Set(['chat']));
  
  // Stats for tags
  const [currentTime, setCurrentTime] = useState('');
  const [todayChatsCount, setTodayChatsCount] = useState(0);
  const [totalSessions, setTotalSessions] = useState(0);
  const [recipesCount] = useState(0); // TODO: Fetch actual recipes count

  // Update time
  useEffect(() => {
    const updateTime = () => {
      const now = new Date();
      setCurrentTime(now.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: true }));
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
      getTag: () => activeSessions.length > 0 ? `${activeSessions.length} active` : `${todayChatsCount} today`,
      tagAlign: 'left',
      hasSubItems: activeSessions.length > 0,
    },
    {
      id: 'history',
      path: '/sessions',
      label: 'History',
      icon: History,
      getTag: () => `${totalSessions}`,
      tagAlign: 'left',
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
        const enabled = extensionsList.filter(ext => ext.enabled).length;
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
    return navItems.find(item => item.id === id);
  };

  // Handle escape key to close overlay
  useEffect(() => {
    if (!(navigationMode === 'overlay' && isNavExpanded)) {
      return;
    }
    
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isNavExpanded && navigationMode === 'overlay') {
        e.preventDefault();
        setIsNavExpanded(false);
      }
    };

    document.addEventListener('keydown', handleKeyDown, { capture: true });
    return () => document.removeEventListener('keydown', handleKeyDown, { capture: true });
  }, [isNavExpanded, navigationMode, setIsNavExpanded]);

  // Track session for /pair navigation
  const [searchParams] = useSearchParams();
  const chatContext = useChatContext();
  const lastSessionIdRef = useRef<string | null>(null);
  const currentSessionId = location.pathname === '/pair' ? searchParams.get('resumeSessionId') : null;

  // Use setView for navigation to pair view (matches original AppSidebar behavior)
  const setView = useNavigation();

  // Keep track of last session ID
  useEffect(() => {
    if (currentSessionId) {
      lastSessionIdRef.current = currentSessionId;
    }
  }, [currentSessionId]);

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

  const isOverlayMode = navigationMode === 'overlay';

  // Truncate session message for display
  const truncateMessage = (msg?: string, maxLen = 24) => {
    if (!msg) return 'New Chat';
    return msg.length > maxLen ? msg.substring(0, maxLen) + '...' : msg;
  };

  const navContent = (
    <motion.div
      initial={{ opacity: 0, scale: 0.95 }}
      animate={{ opacity: 1, scale: 1 }}
      exit={{ opacity: 0, scale: 0.95 }}
      transition={{ type: "spring", stiffness: 350, damping: 25 }}
      className={cn(
        'bg-app rounded-2xl p-4',
        isOverlayMode && 'backdrop-blur-md shadow-2xl',
        className
      )}
    >
      {/* Navigation grid - square tiles */}
      <div className="grid grid-cols-3 sm:grid-cols-4 gap-3">
        {visibleItems.map((item, index) => {
          const Icon = item.icon;
          const active = isActive(item.path);
          const isDragging = draggedItem === item.id;
          const isDragOver = dragOverItem === item.id;
          const isItemExpanded = expandedItems.has(item.id);
          const hasActiveSessions = item.id === 'chat' && activeSessions.length > 0;

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
                opacity: isDragging ? 0.5 : 1, 
                y: 0, 
                scale: isDragging ? 0.95 : 1,
              }}
              transition={{
                type: "spring",
                stiffness: 350,
                damping: 25,
                delay: index * 0.03,
              }}
              className={cn(
                'relative cursor-move group',
                isDragOver && 'ring-2 ring-blue-500 rounded-2xl',
                hasActiveSessions && isItemExpanded && 'col-span-2 row-span-2'
              )}
            >
              <motion.div
                className={cn(
                  'w-full relative flex flex-col',
                  'rounded-2xl',
                  'transition-colors duration-200',
                  hasActiveSessions && isItemExpanded ? 'h-full' : 'aspect-square',
                  active
                    ? 'bg-background-accent text-text-on-accent'
                    : 'bg-background-subtle hover:bg-background-medium'
                )}
              >
                {/* Main button area */}
                <button
                  onClick={() => {
                    if (hasActiveSessions) {
                      toggleExpanded(item.id);
                    } else {
                      handleNavClick(item.path);
                    }
                  }}
                  className="flex-1 flex flex-col items-start justify-between p-5 no-drag text-left"
                >
                  {/* Drag handle */}
                  <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity z-10">
                    <GripVertical className="w-4 h-4 text-text-muted" />
                  </div>

                  {/* Expand/collapse indicator for items with sub-items */}
                  {hasActiveSessions && (
                    <div className="absolute top-2 left-2">
                      {isItemExpanded ? (
                        <ChevronDown className="w-4 h-4 text-text-muted" />
                      ) : (
                        <ChevronRight className="w-4 h-4 text-text-muted" />
                      )}
                    </div>
                  )}

                  {/* Tag/Badge */}
                  {item.getTag && (
                    <div className={cn(
                      'absolute top-3 px-2 py-1 rounded-full',
                      item.tagAlign === 'left' ? 'left-8' : 'right-8',
                      'bg-background-muted'
                    )}>
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
                </button>

                {/* Active sessions sub-items */}
                {hasActiveSessions && isItemExpanded && (
                  <div className="bg-background-subtle pb-3">
                    <div className="h-px bg-border-subtle mb-2 mx-3" />
                    {activeSessions.map((session) => (
                      <button
                        key={session.sessionId}
                        onClick={() => handleSessionClick(session.sessionId)}
                        className={cn(
                          'w-full text-left px-5 py-2 text-sm',
                          'hover:bg-background-medium transition-colors',
                          'flex items-center gap-2'
                        )}
                      >
                        <MessageSquare className="w-3 h-3 flex-shrink-0 text-text-muted" />
                        <span className="truncate text-text-default">
                          {truncateMessage(session.initialMessage)}
                        </span>
                      </button>
                    ))}
                    {/* New chat button */}
                    <button
                      onClick={handleNewChat}
                      className={cn(
                        'w-full text-left px-5 py-2 text-sm',
                        'hover:bg-background-medium transition-colors',
                        'flex items-center gap-2 text-text-muted'
                      )}
                    >
                      <span className="w-3 h-3 flex-shrink-0 text-center">+</span>
                      <span>New Chat</span>
                    </button>
                  </div>
                )}
              </motion.div>
            </motion.div>
          );
        })}
      </div>
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
            
            {/* Centered navigation panel */}
            <div className="absolute inset-0 flex items-center justify-center p-8 pointer-events-none">
              <div className="pointer-events-auto max-w-3xl w-full">
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
