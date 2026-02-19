import { AppEvents } from '../../constants/events';
import React, { useEffect, useState } from 'react';
import {
  AppWindow,
  ChefHat,
  ChevronRight,
  Clock,
  FileText,
  History,
  Home,
  MessageSquarePlus,
  Puzzle,
} from 'lucide-react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import {
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarSeparator,
} from '../ui/sidebar';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '../ui/collapsible';
import { Gear } from '../icons';
import { View, ViewOptions } from '../../utils/navigationUtils';
import { DEFAULT_CHAT_TITLE, useChatContext } from '../../contexts/ChatContext';
import { listSessions, Session, updateSessionName } from '../../api';
import { resumeSession, startNewSession, shouldShowNewChatTitle } from '../../sessions';
import { useNavigation } from '../../hooks/useNavigation';
import { SessionIndicators } from '../SessionIndicators';
import { useSidebarSessionStatus } from '../../hooks/useSidebarSessionStatus';
import { getInitialWorkingDir } from '../../utils/workingDir';
import { useConfig } from '../ConfigContext';
import { InlineEditText } from '../common/InlineEditText';

interface SidebarProps {
  onSelectSession: (sessionId: string) => void;
  refreshTrigger?: number;
  children?: React.ReactNode;
  setView?: (view: View, viewOptions?: ViewOptions) => void;
  currentPath?: string;
}

interface NavigationItem {
  type: 'item';
  path: string;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  tooltip: string;
}

interface NavigationSeparator {
  type: 'separator';
}

type NavigationEntry = NavigationItem | NavigationSeparator;

const menuItems: NavigationEntry[] = [
  {
    type: 'item',
    path: '/recipes',
    label: 'Recipes',
    icon: FileText,
    tooltip: 'Browse your saved recipes',
  },
  {
    type: 'item',
    path: '/apps',
    label: 'Apps',
    icon: AppWindow,
    tooltip: 'MCP and custom apps',
  },
  {
    type: 'item',
    path: '/schedules',
    label: 'Scheduler',
    icon: Clock,
    tooltip: 'Manage scheduled runs',
  },
  {
    type: 'item',
    path: '/extensions',
    label: 'Extensions',
    icon: Puzzle,
    tooltip: 'Manage your extensions',
  },
  { type: 'separator' },
  {
    type: 'item',
    path: '/settings',
    label: 'Settings',
    icon: Gear,
    tooltip: 'Configure Goose settings',
  },
];

const getSessionDisplayName = (session: Session): string => {
  if (session.recipe?.title) {
    return session.recipe.title;
  }

  if (shouldShowNewChatTitle(session)) {
    return DEFAULT_CHAT_TITLE;
  }
  return session.name;
};

const SessionList = React.memo<{
  sessions: Session[];
  activeSessionId: string | undefined;
  getSessionStatus: (
    sessionId: string
  ) => { streamState: string; hasUnreadActivity: boolean } | undefined;
  onSessionClick: (session: Session) => void;
}>(
  ({ sessions, activeSessionId, getSessionStatus, onSessionClick }) => {
    const sortedSessions = React.useMemo(() => {
      return [...sessions].sort((a, b) => {
        const aIsEmptyNew = shouldShowNewChatTitle(a);
        const bIsEmptyNew = shouldShowNewChatTitle(b);
        if (aIsEmptyNew && !bIsEmptyNew) return -1;
        if (!aIsEmptyNew && bIsEmptyNew) return 1;
        return 0;
      });
    }, [sessions]);

    const handleRenameSession = async (sessionId: string, newName: string) => {
      await updateSessionName({
        path: { session_id: sessionId },
        body: { name: newName },
        throwOnError: true,
      });

      // Dispatch event to update all components
      window.dispatchEvent(
        new CustomEvent(AppEvents.SESSION_RENAMED, {
          detail: { sessionId, newName },
        })
      );
    };

    return (
      <div className="relative ml-3">
        {sortedSessions.map((session, index) => {
          const status = getSessionStatus(session.id);
          const isStreaming = status?.streamState === 'streaming';
          const hasError = status?.streamState === 'error';
          const hasUnread = status?.hasUnreadActivity ?? false;
          const displayName = getSessionDisplayName(session);
          const isLast = index === sortedSessions.length - 1;
          const canRename = !session.recipe?.title;

          return (
            <div key={session.id} className="relative flex items-center">
              {/* Vertical line segment - full height except last item stops at middle */}
              <div
                className={`absolute left-0 w-px bg-border-strong ${
                  isLast ? 'top-0 h-1/2' : 'top-0 h-full'
                }`}
              />
              {/* Horizontal branch line */}
              <div className="absolute left-0 w-2 h-px bg-border-strong top-1/2" />
              <button
                onClick={() => onSessionClick(session)}
                className={`w-full text-left ml-3 px-1.5 py-1.5 pr-2 rounded-md text-sm transition-colors flex items-center gap-1 min-w-0 ${
                  activeSessionId === session.id
                    ? 'bg-background-medium text-text-default'
                    : 'text-text-muted hover:bg-background-medium/50 hover:text-text-default'
                }`}
                title={displayName}
              >
                {session.recipe && <ChefHat className="w-3.5 h-3.5 flex-shrink-0" />}
                <div className="flex-1 min-w-0">
                  {canRename ? (
                    <InlineEditText
                      value={displayName}
                      onSave={(newName) => handleRenameSession(session.id, newName)}
                      className="text-sm -mx-2 -my-1"
                      editClassName="text-sm"
                      singleClickEdit={false}
                    />
                  ) : (
                    <span className="truncate block">{displayName}</span>
                  )}
                </div>
                <SessionIndicators
                  isStreaming={isStreaming}
                  hasUnread={hasUnread}
                  hasError={hasError}
                />
              </button>
            </div>
          );
        })}
      </div>
    );
  },
  (prevProps, nextProps) => {
    if (prevProps.sessions.length !== nextProps.sessions.length) return false;
    if (prevProps.activeSessionId !== nextProps.activeSessionId) return false;

    const prevIds = prevProps.sessions.map((s) => s.id).join(',');
    const nextIds = nextProps.sessions.map((s) => s.id).join(',');
    if (prevIds !== nextIds) return false;

    // Check if any session name or message_count changed
    for (let i = 0; i < prevProps.sessions.length; i++) {
      if (prevProps.sessions[i].name !== nextProps.sessions[i].name) return false;
      if (prevProps.sessions[i].message_count !== nextProps.sessions[i].message_count) return false;
    }

    // Check if any session's status has changed
    for (const session of prevProps.sessions) {
      const prevStatus = prevProps.getSessionStatus(session.id);
      const nextStatus = nextProps.getSessionStatus(session.id);

      if (prevStatus?.hasUnreadActivity !== nextStatus?.hasUnreadActivity) return false;
      if (prevStatus?.streamState !== nextStatus?.streamState) return false;
    }

    return true;
  }
);

SessionList.displayName = 'SessionList';

const AppSidebar: React.FC<SidebarProps> = ({ currentPath }) => {
  const navigate = useNavigate();
  const chatContext = useChatContext();
  const configContext = useConfig();
  const setView = useNavigation();

  const appsExtensionEnabled = !!configContext.extensionsList?.find((ext) => ext.name === 'apps')
    ?.enabled;
  const [searchParams] = useSearchParams();
  const [recentSessions, setRecentSessions] = useState<Session[]>([]);
  const [isChatExpanded, setIsChatExpanded] = useState(true);
  const activeSessionId = searchParams.get('resumeSessionId') ?? undefined;
  const { getSessionStatus, clearUnread } = useSidebarSessionStatus(activeSessionId);

  // When activeSessionId changes, ensure it's in the recent sessions list
  // This handles the case where a session is loaded from history that's older than the top 10
  useEffect(() => {
    if (!activeSessionId) return;

    const isInRecentSessions = recentSessions.some((s) => s.id === activeSessionId);
    if (isInRecentSessions) return;

    // Fetch the active session and add it to the top of the list
    const fetchAndAddSession = async () => {
      try {
        const { getSession } = await import('../../api');
        const response = await getSession({ path: { session_id: activeSessionId } });
        if (response.data) {
          setRecentSessions((prev) => {
            // Don't add if it's already there (race condition check)
            if (prev.some((s) => s.id === activeSessionId)) return prev;
            // Add to the beginning and keep max 10
            return [response.data as Session, ...prev].slice(0, 10);
          });
        }
      } catch (error) {
        console.error('Failed to fetch active session:', error);
      }
    };

    fetchAndAddSession();
  }, [activeSessionId, recentSessions]);

  useEffect(() => {
    const loadRecentSessions = async () => {
      try {
        const response = await listSessions<true>({ throwOnError: true });
        const sessions = response.data.sessions.slice(0, 10);
        setRecentSessions(sessions);

        const hasSessionWithDefaultName = sessions.some((s) => shouldShowNewChatTitle(s));

        if (hasSessionWithDefaultName) {
          window.dispatchEvent(new CustomEvent(AppEvents.SESSION_NEEDS_NAME_UPDATE));
        }
      } catch (error) {
        console.error('Failed to load recent sessions:', error);
      }
    };

    loadRecentSessions();
  }, []);

  useEffect(() => {
    let pollingTimeouts: ReturnType<typeof setTimeout>[] = [];
    let isPolling = false;

    const handleSessionCreated = (event: Event) => {
      const { session } = (event as CustomEvent<{ session?: Session }>).detail || {};
      // If session data is provided, add it immediately to the sidebar
      // This is for displaying sessions that won't be returned by the API due to not having messages yet
      if (session) {
        setRecentSessions((prev) => {
          if (prev.some((s) => s.id === session.id)) return prev;
          return [session, ...prev].slice(0, 10);
        });
      }

      // Poll for updates to get the generated session name
      if (isPolling) {
        return;
      }

      isPolling = true;
      const pollIntervalMs = 300;
      const maxPollDurationMs = 10000;
      const maxPolls = maxPollDurationMs / pollIntervalMs;
      let pollCount = 0;

      const pollForUpdates = async () => {
        pollCount++;

        try {
          const response = await listSessions<true>({ throwOnError: true });
          const apiSessions = response.data.sessions.slice(0, 10);

          // Merge API sessions with any locally-tracked empty sessions
          setRecentSessions((prev) => {
            const emptyLocalSessions = prev.filter(
              (local) =>
                local.message_count === 0 && !apiSessions.some((api) => api.id === local.id)
            );
            const merged = [...emptyLocalSessions, ...apiSessions];
            const seen = new Set<string>();
            return merged
              .filter((s) => {
                if (seen.has(s.id)) return false;
                seen.add(s.id);
                return true;
              })
              .slice(0, 10);
          });

          const sessionWithDefaultName = apiSessions.find((s) => shouldShowNewChatTitle(s));

          const shouldContinue = pollCount < maxPolls && (sessionWithDefaultName || pollCount < 5);

          if (shouldContinue) {
            const timeoutId = setTimeout(pollForUpdates, pollIntervalMs);
            pollingTimeouts.push(timeoutId);
          } else {
            isPolling = false;
          }
        } catch {
          isPolling = false;
        }
      };
      pollForUpdates();
    };

    const handleSessionNeedsNameUpdate = () => {
      handleSessionCreated(new CustomEvent(AppEvents.SESSION_CREATED, { detail: {} }));
    };

    const handleSessionDeleted = (event: Event) => {
      const { sessionId } = (event as CustomEvent<{ sessionId: string }>).detail;
      setRecentSessions((prev) => prev.filter((s) => s.id !== sessionId));
    };

    const handleSessionRenamed = (event: Event) => {
      const { sessionId, newName } = (event as CustomEvent<{ sessionId: string; newName: string }>)
        .detail;
      setRecentSessions((prev) =>
        prev.map((s) =>
          s.id === sessionId
            ? { ...s, name: newName, message_count: Math.max(s.message_count, 1) }
            : s
        )
      );
    };

    window.addEventListener(AppEvents.SESSION_CREATED, handleSessionCreated);
    window.addEventListener(AppEvents.SESSION_NEEDS_NAME_UPDATE, handleSessionNeedsNameUpdate);
    window.addEventListener(AppEvents.SESSION_DELETED, handleSessionDeleted);
    window.addEventListener(AppEvents.SESSION_RENAMED, handleSessionRenamed);

    return () => {
      window.removeEventListener(AppEvents.SESSION_CREATED, handleSessionCreated);
      window.removeEventListener(AppEvents.SESSION_NEEDS_NAME_UPDATE, handleSessionNeedsNameUpdate);
      window.removeEventListener(AppEvents.SESSION_DELETED, handleSessionDeleted);
      window.removeEventListener(AppEvents.SESSION_RENAMED, handleSessionRenamed);
      pollingTimeouts.forEach(clearTimeout);
      isPolling = false;
    };
  }, []);

  useEffect(() => {
    const currentItem = menuItems.find(
      (item) => item.type === 'item' && item.path === currentPath
    ) as NavigationItem | undefined;

    const titleBits = ['Goose'];

    if (
      currentPath === '/pair' &&
      chatContext?.chat?.name &&
      chatContext.chat.name !== DEFAULT_CHAT_TITLE
    ) {
      titleBits.push(chatContext.chat.name);
    } else if (currentPath !== '/' && currentItem) {
      titleBits.push(currentItem.label);
    }

    document.title = titleBits.join(' - ');
  }, [currentPath, chatContext?.chat?.name]);

  const isActivePath = (path: string) => {
    return currentPath === path;
  };

  // Use a ref to access the latest recentSessions without causing re-renders or dependency issues
  const recentSessionsRef = React.useRef(recentSessions);
  React.useEffect(() => {
    recentSessionsRef.current = recentSessions;
  }, [recentSessions]);

  // Guard ref to prevent duplicate session creation from key commands
  const isCreatingSessionRef = React.useRef(false);

  const handleNewChat = React.useCallback(async () => {
    if (isCreatingSessionRef.current) {
      return;
    }

    const emptyNewSession = recentSessionsRef.current.find((s) => shouldShowNewChatTitle(s));

    if (emptyNewSession) {
      clearUnread(emptyNewSession.id);
      resumeSession(emptyNewSession, setView);
    } else {
      isCreatingSessionRef.current = true;
      try {
        await startNewSession('', setView, getInitialWorkingDir(), {
          allExtensions: configContext.extensionsList,
        });
      } finally {
        setTimeout(() => {
          isCreatingSessionRef.current = false;
        }, 1000);
      }
    }
  }, [setView, clearUnread, configContext.extensionsList]);

  useEffect(() => {
    const handleTriggerNewChat = () => {
      handleNewChat();
    };

    window.addEventListener(AppEvents.TRIGGER_NEW_CHAT, handleTriggerNewChat);
    return () => {
      window.removeEventListener(AppEvents.TRIGGER_NEW_CHAT, handleTriggerNewChat);
    };
  }, [handleNewChat]);

  const handleSessionClick = React.useCallback(
    async (session: Session) => {
      clearUnread(session.id);
      resumeSession(session, setView);
    },
    [clearUnread, setView]
  );

  const handleViewAllClick = React.useCallback(() => {
    navigate('/sessions');
  }, [navigate]);

  const renderMenuItem = (entry: NavigationEntry, index: number) => {
    if (entry.type === 'separator') {
      return <SidebarSeparator key={index} />;
    }

    const IconComponent = entry.icon;

    return (
      <SidebarGroup key={entry.path} className="px-2">
        <SidebarGroupContent className="space-y-1">
          <div className="sidebar-item">
            <SidebarMenuItem>
              <SidebarMenuButton
                data-testid={`sidebar-${entry.label.toLowerCase()}-button`}
                onClick={() => navigate(entry.path)}
                isActive={isActivePath(entry.path)}
                tooltip={entry.tooltip}
                className="w-full justify-start px-3 rounded-lg h-fit hover:bg-background-medium/50 transition-all duration-200 data-[active=true]:bg-background-medium"
              >
                <IconComponent className="w-4 h-4" />
                <span>{entry.label}</span>
              </SidebarMenuButton>
            </SidebarMenuItem>
          </div>
        </SidebarGroupContent>
      </SidebarGroup>
    );
  };

  const visibleMenuItems = menuItems.filter((entry) => {
    // Filter out Apps if extension is not enabled
    if (entry.type === 'item' && entry.path === '/apps') {
      return appsExtensionEnabled;
    }
    return true;
  });

  return (
    <>
      <SidebarContent className="pt-12">
        <SidebarMenu>
          {/* Home */}
          <SidebarGroup className="px-2">
            <SidebarGroupContent className="space-y-1">
              <div className="sidebar-item">
                <SidebarMenuItem>
                  <SidebarMenuButton
                    data-testid="sidebar-home-button"
                    onClick={() => navigate('/')}
                    isActive={isActivePath('/')}
                    tooltip="Go back to the main chat screen"
                    className="w-full justify-start px-3 rounded-lg h-fit hover:bg-background-medium/50 transition-all duration-200 data-[active=true]:bg-background-medium"
                  >
                    <Home className="w-4 h-4" />
                    <span>Home</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              </div>
            </SidebarGroupContent>
          </SidebarGroup>

          {/* Chat with Collapsible Sessions */}
          <SidebarGroup className="px-2">
            <SidebarGroupContent className="space-y-1">
              <Collapsible open={isChatExpanded} onOpenChange={setIsChatExpanded}>
                <div className="sidebar-item">
                  <SidebarMenuItem>
                    <div className="flex items-center w-full">
                      <SidebarMenuButton
                        data-testid="sidebar-new-chat-button"
                        onClick={handleNewChat}
                        tooltip="Start a new chat"
                        className="flex-1 justify-start px-3 rounded-lg h-fit hover:bg-background-medium/50 transition-all duration-200"
                      >
                        <MessageSquarePlus className="w-4 h-4" />
                        <span>Chat</span>
                      </SidebarMenuButton>
                      {recentSessions.length > 0 && (
                        <CollapsibleTrigger asChild>
                          <button
                            className="flex items-center justify-center w-6 h-8 hover:bg-background-medium/50 rounded-md transition-colors"
                            aria-label={
                              isChatExpanded ? 'Collapse chat sessions' : 'Expand chat sessions'
                            }
                          >
                            <ChevronRight
                              className={`w-4 h-4 text-text-muted transition-transform duration-200 ${
                                isChatExpanded ? 'rotate-90' : ''
                              }`}
                            />
                          </button>
                        </CollapsibleTrigger>
                      )}
                    </div>
                  </SidebarMenuItem>
                </div>
                {recentSessions.length > 0 && (
                  <CollapsibleContent className="overflow-hidden transition-all data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:animate-in data-[state=open]:fade-in-0">
                    <div className="mt-1 space-y-1">
                      <SessionList
                        sessions={recentSessions}
                        activeSessionId={activeSessionId}
                        getSessionStatus={getSessionStatus}
                        onSessionClick={handleSessionClick}
                      />
                      {/* View All Link */}
                      <button
                        onClick={handleViewAllClick}
                        className="w-full text-left px-3 py-1.5 rounded-md text-sm text-text-muted hover:bg-background-medium/50 hover:text-text-default transition-colors flex items-center gap-2"
                      >
                        <History className="w-4 h-4" />
                        <span>View All</span>
                      </button>
                    </div>
                  </CollapsibleContent>
                )}
              </Collapsible>
            </SidebarGroupContent>
          </SidebarGroup>

          <SidebarSeparator />

          {visibleMenuItems.map((entry, index) => renderMenuItem(entry, index))}
        </SidebarMenu>
      </SidebarContent>
    </>
  );
};

export default AppSidebar;
