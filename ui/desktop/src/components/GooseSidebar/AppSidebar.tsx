import React, { useEffect, useState } from 'react';
import { Clock, FileText, History, Home, Plus, Puzzle } from 'lucide-react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import {
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarSeparator,
} from '../ui/sidebar';
import { Gear } from '../icons';
import { View, ViewOptions } from '../../utils/navigationUtils';
import { DEFAULT_CHAT_TITLE, useChatContext } from '../../contexts/ChatContext';
import EnvironmentBadge from './EnvironmentBadge';
import { listSessions, Session } from '../../api';
import { resumeSession, startNewSession } from '../../sessions';
import { useNavigation } from '../../hooks/useNavigation';
import { SessionIndicators } from '../SessionIndicators';
import { useSessionStatusContext } from '../../contexts/SessionStatusContext';
import { isDefaultSessionName } from '../../sessions';
import { getInitialWorkingDir } from '../../utils/workingDir';

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

const SessionList = React.memo<{
  sessions: Session[];
  activeSessionId: string | undefined;
  getSessionStatus: (
    sessionId: string
  ) => { streamState: string; hasUnreadActivity: boolean } | undefined;
  onSessionClick: (session: Session) => void;
}>(
  ({ sessions, activeSessionId, getSessionStatus, onSessionClick }) => {
    return (
      <>
        {sessions.map((session) => {
          const status = getSessionStatus(session.id);
          const isStreaming = status?.streamState === 'streaming';
          const hasError = status?.streamState === 'error';
          const hasUnread = status?.hasUnreadActivity ?? false;
          const displayName = isDefaultSessionName(session.name)
            ? DEFAULT_CHAT_TITLE
            : session.name;

          return (
            <button
              key={session.id}
              onClick={() => onSessionClick(session)}
              className={`w-full text-left px-3 py-1.5 pr-2 rounded-md text-sm transition-colors flex items-center gap-2 min-w-0 ${
                activeSessionId === session.id
                  ? 'bg-background-medium text-text-default'
                  : 'text-text-muted hover:bg-background-medium/50 hover:text-text-default'
              }`}
              title={displayName}
            >
              <span className="flex-1 truncate min-w-0 block">{displayName}</span>
              <SessionIndicators
                isStreaming={isStreaming}
                hasUnread={hasUnread}
                hasError={hasError}
              />
            </button>
          );
        })}
      </>
    );
  },
  (prevProps, nextProps) => {
    if (prevProps.sessions.length !== nextProps.sessions.length) return false;
    if (prevProps.activeSessionId !== nextProps.activeSessionId) return false;

    const prevIds = prevProps.sessions.map((s) => s.id).join(',');
    const nextIds = nextProps.sessions.map((s) => s.id).join(',');
    if (prevIds !== nextIds) return false;

    // Check if any session name changed
    for (let i = 0; i < prevProps.sessions.length; i++) {
      if (prevProps.sessions[i].name !== nextProps.sessions[i].name) return false;
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
  const setView = useNavigation();
  const [searchParams] = useSearchParams();
  const [recentSessions, setRecentSessions] = useState<Session[]>([]);
  const [isLoadingSessions, setIsLoadingSessions] = useState(false);
  const { getSessionStatus, markSessionActive, trackSession } = useSessionStatusContext();
  const activeSessionId = searchParams.get('resumeSessionId') ?? undefined;

  useEffect(() => {
    recentSessions.forEach((session) => {
      trackSession(session.id);
    });
  }, [recentSessions, trackSession]);

  useEffect(() => {
    const timer = setTimeout(() => {
      // setIsVisible(true);
    }, 100);

    return () => clearTimeout(timer);
  }, []);

  useEffect(() => {
    const loadRecentSessions = async () => {
      setIsLoadingSessions(true);
      try {
        const response = await listSessions<true>({ throwOnError: true });
        const sessions = response.data.sessions.slice(0, 10);
        setRecentSessions(sessions);

        const hasSessionWithDefaultName = sessions.some((s) => isDefaultSessionName(s.name));

        if (hasSessionWithDefaultName) {
          window.dispatchEvent(new CustomEvent('session-needs-name-update'));
        }
      } catch (error) {
        console.error('Failed to load recent sessions:', error);
      } finally {
        setIsLoadingSessions(false);
      }
    };

    loadRecentSessions();
  }, []);

  useEffect(() => {
    let pollingTimeouts: ReturnType<typeof setTimeout>[] = [];
    let isPolling = false;

    const handleSessionCreated = () => {
      if (isPolling) {
        return;
      }

      isPolling = true;
      const pollIntervalMs = 300; // Poll every 300ms
      const maxPollDurationMs = 10000; // Poll for up to 10 seconds
      const maxPolls = maxPollDurationMs / pollIntervalMs;
      let pollCount = 0;

      const pollForUpdates = async () => {
        pollCount++;

        try {
          const response = await listSessions<true>({ throwOnError: true });
          const sessions = response.data.sessions.slice(0, 10);
          setRecentSessions(sessions);

          const sessionWithDefaultName = sessions.find((s) => isDefaultSessionName(s.name));

          const shouldContinue = pollCount < maxPolls && (sessionWithDefaultName || pollCount < 5);

          if (shouldContinue) {
            const timeoutId = setTimeout(pollForUpdates, pollIntervalMs);
            pollingTimeouts.push(timeoutId);
          } else {
            isPolling = false;
          }
          // eslint-disable-next-line @typescript-eslint/no-unused-vars
        } catch (error) {
          isPolling = false;
        }
      };
      pollForUpdates();
    };

    window.addEventListener('session-created', handleSessionCreated);
    window.addEventListener('session-needs-name-update', handleSessionCreated);

    return () => {
      window.removeEventListener('session-created', handleSessionCreated);
      window.removeEventListener('session-needs-name-update', handleSessionCreated);
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

  const handleNewChat = React.useCallback(async () => {
    await startNewSession('', setView, getInitialWorkingDir());
  }, [setView]);

  const handleSessionClick = React.useCallback(
    async (session: Session) => {
      markSessionActive(session.id);
      resumeSession(session, setView);
    },
    [markSessionActive, setView]
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

  return (
    <>
      <SidebarContent className="pt-16">
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

          <SidebarSeparator />

          {/* New Chat Button */}
          <SidebarGroup className="px-2">
            <SidebarGroupContent className="space-y-1">
              <div className="sidebar-item">
                <SidebarMenuItem>
                  <SidebarMenuButton
                    data-testid="sidebar-new-chat-button"
                    onClick={handleNewChat}
                    tooltip="Start a new chat"
                    className="w-full justify-start px-3 rounded-lg h-fit hover:bg-background-medium/50 transition-all duration-200"
                  >
                    <Plus className="w-4 h-4" />
                    <span>New Chat</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              </div>
            </SidebarGroupContent>
          </SidebarGroup>

          {/* Recent Sessions */}
          <SidebarGroup className="px-2">
            <SidebarGroupContent className="space-y-1">
              {isLoadingSessions ? (
                <div className="text-xs text-text-muted px-3 py-1">Loading...</div>
              ) : recentSessions.length > 0 ? (
                <>
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
                </>
              ) : (
                <div className="text-xs text-text-muted px-3 py-1">No sessions yet</div>
              )}
            </SidebarGroupContent>
          </SidebarGroup>

          <SidebarSeparator />

          {/* Other menu items */}
          {menuItems.map((entry, index) => renderMenuItem(entry, index))}
        </SidebarMenu>
      </SidebarContent>

      <SidebarFooter className="pb-2 flex items-start">
        <EnvironmentBadge />
      </SidebarFooter>
    </>
  );
};

export default AppSidebar;
