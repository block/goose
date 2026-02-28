import * as Popover from '@radix-ui/react-popover';
import {
  Activity,
  AppWindow,
  Bot,
  ChefHat,
  ChevronRight,
  Clock,
  FileText,
  FlaskConical,
  FolderOpen,
  FolderPlus,
  History,
  Home,
  Pin,
  PinOff,
  Plus,
  Puzzle,
  Search,
  Trash2,
  Workflow,
  X,
} from 'lucide-react';
import gooseIcon from '@/images/icon.svg';

import React, { useEffect, useState } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { deleteSession, listSessions, updateSessionName, type Session } from '@/api';
import { AppEvents } from '@/constants/events';
import { DEFAULT_CHAT_TITLE, useChatContext } from '@/contexts/ChatContext';
import { useConfig } from '@/contexts/ConfigContext';
import { useNavigation } from '@/hooks/useNavigation';
import { useProjectPreferences } from '@/hooks/useProjectPreferences';
import { useSidebarSessionStatus } from '@/hooks/useSidebarSessionStatus';
import { resumeSession, shouldShowNewChatTitle, startNewSession } from '@/sessions';
import type { View, ViewOptions } from '@/utils/navigationUtils';
import { getInitialWorkingDir } from '@/utils/workingDir';
import { homeDir } from '@/utils/homeDir';
import { InlineEditText } from '../common/InlineEditText';
import { Gear } from '@/components/atoms/icons';
import { SessionIndicators } from '../shared/SessionIndicators';
import { UserAvatarMenu } from '../shared/UserAvatarMenu';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/molecules/ui/collapsible';
import {
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuAction,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarSeparator,
} from '@/components/molecules/ui/sidebar';

interface SidebarProps {
  onSelectSession: (sessionId: string) => void;
  refreshTrigger?: number;
  children?: React.ReactNode;
  setView?: (view: View, viewOptions?: ViewOptions) => void;
  currentPath?: string;
}

interface NavigationItem {
  path: string;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  tooltip: string;
  condition?: string;
}

interface NavigationZone {
  id: string;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  items: NavigationItem[];
  route?: string;
}

// Zone-based navigation: Workflows → Monitoring → Evaluate → Catalog → Settings
const navigationZones: NavigationZone[] = [
  {
    id: 'workflows',
    label: 'Workflows',
    icon: Workflow,
    route: '/workflows',
    items: [
      {
        path: '/recipes',
        label: 'Recipes',
        icon: FileText,
        tooltip: 'Browse and run installed workflows',
      },
      {
        path: '/schedules',
        label: 'Scheduler',
        icon: Clock,
        tooltip: 'Manage scheduled runs',
      },
      {
        path: '/pipelines',
        label: 'Pipelines',
        icon: Workflow,
        tooltip: 'Visual DAG workflow builder',
      },
    ],
  },
  {
    id: 'monitoring',
    label: 'Monitoring',
    icon: Activity,
    items: [
      {
        path: '/monitoring',
        label: 'Monitoring',
        icon: Activity,
        tooltip: 'View usage dashboard and live metrics',
      },
    ],
  },
  {
    id: 'evaluate',
    label: 'Evaluate',
    icon: FlaskConical,
    items: [
      {
        path: '/evaluate',
        label: 'Evaluate',
        icon: FlaskConical,
        tooltip: 'Eval overview, datasets, runs, and topics',
      },
    ],
  },
  {
    id: 'catalog',
    label: 'Catalog',
    icon: Puzzle,
    route: '/catalogs',
    items: [
      {
        path: '/agents',
        label: 'Agents',
        icon: Bot,
        tooltip: 'Browse and manage agent personas',
      },
      {
        path: '/extensions',
        label: 'Extensions',
        icon: Puzzle,
        tooltip: 'Browse and install tool extensions',
      },
      {
        path: '/apps',
        label: 'Apps',
        icon: AppWindow,
        tooltip: 'MCP and custom apps',
        condition: 'apps',
      },
    ],
  },
  {
    id: 'settings',
    label: 'Settings',
    icon: Gear,
    items: [
      {
        path: '/settings',
        label: 'Settings',
        icon: Gear,
        tooltip: 'Configure Goose settings',
      },
    ],
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

const getProjectName = (workingDir: string | undefined | null): string => {
  if (!workingDir) return 'General';

  // Temp dirs (/tmp/.tmpXXXXXX) → group as "General"
  if (workingDir.startsWith('/tmp/.tmp') || workingDir.startsWith('/tmp/tmp')) {
    return 'General';
  }

  // User home dir with no project context → General
  // Detect home dir from path patterns since appConfig doesn't expose HOME
  if (workingDir === '~') return 'General';
  if (/^\/home\/[^/]+$/.test(workingDir) || /^\/Users\/[^/]+$/.test(workingDir)) {
    return 'General';
  }

  const cleanPath = workingDir.replace(/\/$/, '');
  const parts = cleanPath.split('/');
  const name = parts[parts.length - 1] || 'General';

  // Disambiguate nested subdirs of the same parent project
  // e.g. /home/user/codes/goose4 → "goose4"
  //      /home/user/codes/goose4/crates/goose → "goose4 › crates/goose"
  // Find all sessions' common ancestor to detect shared parent projects
  const initialWorkDir = typeof window !== 'undefined' ? getInitialWorkingDir() : '';
  if (initialWorkDir) {
    const iwdClean = initialWorkDir.replace(/\/$/, '');
    if (cleanPath !== iwdClean && cleanPath.startsWith(`${iwdClean}/`)) {
      const iwdName = iwdClean.split('/').pop() || '';
      const relative = cleanPath.slice(iwdClean.length + 1);
      return `${iwdName} › ${relative}`;
    }
  }

  return name;
};

interface ProjectGroup {
  project: string;
  sessions: Session[];
}

const MAX_VISIBLE_PROJECTS = 10;

const groupSessionsByProject = (
  sessions: Session[],
  pinnedProjects: string[] = []
): ProjectGroup[] => {
  const groups = new Map<string, Session[]>();

  for (const session of sessions) {
    const project = getProjectName(session.working_dir);
    const existing = groups.get(project) || [];
    existing.push(session);
    groups.set(project, existing);
  }

  // Ensure "General" group always exists (even if empty)
  if (!groups.has('General')) {
    groups.set('General', []);
  }

  return Array.from(groups.entries())
    .map(([project, projectSessions]) => ({ project, sessions: projectSessions }))
    .sort((a, b) => {
      // General always first
      if (a.project === 'General') return -1;
      if (b.project === 'General') return 1;
      // Then pinned projects
      const aPinned = pinnedProjects.includes(a.project);
      const bPinned = pinnedProjects.includes(b.project);
      if (aPinned && !bPinned) return -1;
      if (!aPinned && bPinned) return 1;
      return 0;
    });
};

const SessionItem: React.FC<{
  session: Session;
  isLast: boolean;
  activeSessionId: string | undefined;
  getSessionStatus: (
    sessionId: string
  ) => { streamState: string; hasUnreadActivity: boolean } | undefined;
  onSessionClick: (session: Session) => void;
  onDeleteSession?: (sessionId: string) => void;
}> = ({ session, isLast, activeSessionId, getSessionStatus, onSessionClick, onDeleteSession }) => {
  const status = getSessionStatus(session.id);
  const isStreaming = status?.streamState === 'streaming';
  const hasError = status?.streamState === 'error';
  const hasUnread = status?.hasUnreadActivity ?? false;
  const displayName = getSessionDisplayName(session);
  const canRename = !session.recipe?.title;

  const handleRenameSession = async (sessionId: string, newName: string) => {
    await updateSessionName({
      path: { session_id: sessionId },
      body: { name: newName },
      throwOnError: true,
    });
    window.dispatchEvent(
      new CustomEvent(AppEvents.SESSION_RENAMED, { detail: { sessionId, newName } })
    );
  };

  const handleDelete = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (isStreaming) return;
    onDeleteSession?.(session.id);
  };

  return (
    <div className="relative flex items-center group/session">
      <div
        className={`absolute left-0 w-px bg-border-strong ${
          isLast ? 'top-0 h-1/2' : 'top-0 h-full'
        }`}
      />
      <div className="absolute left-0 w-2 h-px bg-border-strong top-1/2" />

      {/*
        The session row is "click anywhere" for navigation, but also contains interactive
        controls (rename, delete). Use an overlay button to avoid nesting interactive
        elements (e.g. InlineEditText renders a button/input).
      */}
      <div
        className={`relative z-10 w-full text-left ml-3 px-1.5 py-1.5 pr-7 rounded-md text-sm flex items-center gap-1 min-w-0 rounded-md transition-colors cursor-pointer ${
          activeSessionId === session.id
            ? 'bg-background-medium text-text-default'
            : 'text-text-muted hover:bg-background-medium/50 hover:text-text-default'
        }`}
        title={displayName}
        role="button"
        tabIndex={0}
        aria-label={`Open session ${displayName}`}
        onClick={() => onSessionClick(session)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            onSessionClick(session);
          }
        }}
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

        <SessionIndicators isStreaming={isStreaming} hasUnread={hasUnread} hasError={hasError} />
      </div>

      {onDeleteSession && !isStreaming && (
        <button
          type="button"
          onClick={handleDelete}
          className="absolute right-1 top-1/2 -translate-y-1/2 z-20 opacity-0 group-hover/session:opacity-100 p-1 hover:bg-background-danger-muted rounded transition-all"
          title="Delete session"
        >
          <Trash2 className="w-3 h-3 text-text-danger" />
        </button>
      )}
    </div>
  );
};

const SessionList = React.memo<{
  sessions: Session[];
  activeSessionId: string | undefined;
  getSessionStatus: (
    sessionId: string
  ) => { streamState: string; hasUnreadActivity: boolean } | undefined;
  onSessionClick: (session: Session) => void;
  onDeleteSession: (sessionId: string) => void;
  onCloseProject: (projectName: string, sessionIds: string[]) => void;
  onNewSessionInProject?: (projectDir: string) => void;
}>(
  ({
    sessions,
    activeSessionId,
    getSessionStatus,
    onSessionClick,
    onDeleteSession,
    onCloseProject,
    onNewSessionInProject,
  }) => {
    const { pinnedProjects, togglePin, isPinned, toggleCollapsed, isCollapsed } =
      useProjectPreferences();
    const [projectSearch, setProjectSearch] = useState('');

    const sortedSessions = React.useMemo(() => {
      return [...sessions].sort((a, b) => {
        const aIsEmptyNew = shouldShowNewChatTitle(a);
        const bIsEmptyNew = shouldShowNewChatTitle(b);
        if (aIsEmptyNew && !bIsEmptyNew) return -1;
        if (!aIsEmptyNew && bIsEmptyNew) return 1;
        return 0;
      });
    }, [sessions]);

    const projectGroups = React.useMemo(
      () => groupSessionsByProject(sortedSessions, pinnedProjects),
      [sortedSessions, pinnedProjects]
    );

    const shouldGroup = projectGroups.length > 1 || sessions.length >= 5;

    const filteredGroups = React.useMemo(() => {
      if (!projectSearch.trim()) return projectGroups.slice(0, MAX_VISIBLE_PROJECTS);
      const q = projectSearch.toLowerCase();
      return projectGroups.filter((g) => g.project.toLowerCase().includes(q));
    }, [projectGroups, projectSearch]);

    const hasOverflow = projectGroups.length > MAX_VISIBLE_PROJECTS && !projectSearch;

    if (!shouldGroup) {
      return (
        <div className="relative ml-3">
          {sortedSessions.map((session, index) => (
            <SessionItem
              key={session.id}
              session={session}
              isLast={index === sortedSessions.length - 1}
              activeSessionId={activeSessionId}
              getSessionStatus={getSessionStatus}
              onSessionClick={onSessionClick}
              onDeleteSession={onDeleteSession}
            />
          ))}
        </div>
      );
    }

    return (
      <div className="space-y-1">
        {(hasOverflow || projectSearch) && (
          <div className="px-2 pb-1">
            <div className="relative">
              <Search className="absolute left-2 top-1/2 -translate-y-1/2 w-3 h-3 text-text-subtle" />
              <input
                type="text"
                value={projectSearch}
                onChange={(e) => setProjectSearch(e.target.value)}
                placeholder="Find project..."
                className="w-full pl-6 pr-2 py-1 text-xs bg-background-muted border border-border-muted rounded text-text-default placeholder-text-subtle focus:outline-none focus:border-border-default"
              />
            </div>
          </div>
        )}
        {filteredGroups.map((group) => {
          const collapsed = isCollapsed(group.project);
          const pinned = isPinned(group.project);
          const isGeneral = group.project === 'General';

          // General sessions render directly (no header) — they belong to the "New Chat" section above
          if (isGeneral) {
            return (
              <div key={group.project} className="relative ml-1">
                {group.sessions.map((session, index) => (
                  <SessionItem
                    key={session.id}
                    session={session}
                    isLast={index === group.sessions.length - 1}
                    activeSessionId={activeSessionId}
                    getSessionStatus={getSessionStatus}
                    onSessionClick={onSessionClick}
                    onDeleteSession={onDeleteSession}
                  />
                ))}
              </div>
            );
          }

          return (
            <div key={group.project}>
              <div className="flex items-center group/project">
                <button type="button"
                  onClick={() => toggleCollapsed(group.project)}
                  className="flex items-center gap-1.5 flex-1 min-w-0 px-2 py-1 text-xs font-medium text-text-muted hover:text-text-default transition-colors rounded-md hover:bg-background-medium/30"
                >
                  <FolderOpen className="w-3 h-3 flex-shrink-0" />
                  <span className="truncate">{group.project}</span>
                  {pinned && <Pin className="w-2.5 h-2.5 flex-shrink-0 text-text-accent" />}
                  <span className="text-[10px] opacity-60 ml-auto flex-shrink-0">
                    {group.sessions.length}
                  </span>
                  <ChevronRight
                    className={`w-3 h-3 flex-shrink-0 transition-transform duration-200 ${!collapsed ? 'rotate-90' : ''}`}
                  />
                </button>
                {/* Hover action strip — inline icons that appear on hover */}
                <div className="flex-shrink-0 flex items-center gap-0 opacity-0 pointer-events-none group-hover/project:opacity-100 group-hover/project:pointer-events-auto transition-opacity duration-150">
                  {onNewSessionInProject && (
                    <button type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        onNewSessionInProject(group.sessions[0]?.working_dir || '');
                      }}
                      className="p-1 hover:bg-background-muted rounded transition-colors"
                      title="New session in project"
                    >
                      <Plus className="w-3.5 h-3.5 text-text-muted hover:text-text-default" />
                    </button>
                  )}
                  <button type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      togglePin(group.project);
                    }}
                    className="p-1 hover:bg-background-muted rounded transition-colors"
                    title={pinned ? 'Unpin project' : 'Pin project'}
                  >
                    {pinned ? (
                      <PinOff className="w-3.5 h-3.5 text-text-muted hover:text-text-default" />
                    ) : (
                      <Pin className="w-3.5 h-3.5 text-text-muted hover:text-text-default" />
                    )}
                  </button>
                  <button type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      onCloseProject(
                        group.project,
                        group.sessions.map((s) => s.id)
                      );
                    }}
                    className="p-1 hover:bg-background-danger/10 rounded transition-colors"
                    title="Close project"
                  >
                    <X className="w-3.5 h-3.5 text-text-danger" />
                  </button>
                </div>
              </div>
              {!collapsed && (
                <div className="relative ml-3">
                  {group.sessions.map((session, index) => (
                    <SessionItem
                      key={session.id}
                      session={session}
                      isLast={index === group.sessions.length - 1}
                      activeSessionId={activeSessionId}
                      getSessionStatus={getSessionStatus}
                      onSessionClick={onSessionClick}
                      onDeleteSession={onDeleteSession}
                    />
                  ))}
                </div>
              )}
            </div>
          );
        })}
        {hasOverflow && (
          <div className="px-2 text-[10px] text-text-subtle">
            {projectGroups.length - MAX_VISIBLE_PROJECTS} more projects — use search to find them
          </div>
        )}
      </div>
    );
  },
  (prevProps, nextProps) => {
    if (prevProps.sessions.length !== nextProps.sessions.length) return false;
    if (prevProps.activeSessionId !== nextProps.activeSessionId) return false;

    const prevIds = prevProps.sessions.map((s) => s.id).join(',');
    const nextIds = nextProps.sessions.map((s) => s.id).join(',');
    if (prevIds !== nextIds) return false;

    for (let i = 0; i < prevProps.sessions.length; i++) {
      if (prevProps.sessions[i].name !== nextProps.sessions[i].name) return false;
      if (prevProps.sessions[i].message_count !== nextProps.sessions[i].message_count) return false;
    }

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
  const location = useLocation();
  const [recentSessions, setRecentSessions] = useState<Session[]>([]);

  const activeSessionId = React.useMemo(() => {
    const match = location.pathname.match(/^\/sessions\/([^/]+)$/);
    if (!match) return undefined;
    try {
      return decodeURIComponent(match[1]);
    } catch {
      return match[1];
    }
  }, [location.pathname]);
  const { getSessionStatus, clearUnread } = useSidebarSessionStatus(activeSessionId);
  const { addRecentDir, recentDirs } = useProjectPreferences();
  const [projectDropdownOpen, setProjectDropdownOpen] = useState(false);
  // This handles the case where a session is loaded from history that's older than the top 10
  useEffect(() => {
    if (!activeSessionId) return;

    const isInRecentSessions = recentSessions.some((s) => s.id === activeSessionId);
    if (isInRecentSessions) return;

    // Fetch the active session and add it to the top of the list
    const fetchAndAddSession = async () => {
      try {
        const { getSession } = await import('@/api');
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
    const pollingTimeouts: ReturnType<typeof setTimeout>[] = [];
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

    const handleSessionWorkingDirChanged = (event: Event) => {
      const { sessionId, workingDir } = (
        event as CustomEvent<{ sessionId: string; workingDir: string }>
      ).detail;
      setRecentSessions((prev) =>
        prev.map((s) => (s.id === sessionId ? { ...s, working_dir: workingDir } : s))
      );
    };

    window.addEventListener(AppEvents.SESSION_CREATED, handleSessionCreated);
    window.addEventListener(AppEvents.SESSION_NEEDS_NAME_UPDATE, handleSessionNeedsNameUpdate);
    window.addEventListener(AppEvents.SESSION_DELETED, handleSessionDeleted);
    window.addEventListener(AppEvents.SESSION_RENAMED, handleSessionRenamed);
    window.addEventListener(AppEvents.SESSION_WORKING_DIR_CHANGED, handleSessionWorkingDirChanged);

    return () => {
      window.removeEventListener(AppEvents.SESSION_CREATED, handleSessionCreated);
      window.removeEventListener(AppEvents.SESSION_NEEDS_NAME_UPDATE, handleSessionNeedsNameUpdate);
      window.removeEventListener(AppEvents.SESSION_DELETED, handleSessionDeleted);
      window.removeEventListener(AppEvents.SESSION_RENAMED, handleSessionRenamed);
      window.removeEventListener(
        AppEvents.SESSION_WORKING_DIR_CHANGED,
        handleSessionWorkingDirChanged
      );
      pollingTimeouts.forEach(clearTimeout);
      isPolling = false;
    };
  }, []);

  // Find current navigation item across all zones for window title
  useEffect(() => {
    const allItems = navigationZones.flatMap((z) => z.items);
    const currentItem = allItems.find((item) => item.path === currentPath);

    const titleBits = ['Goose'];

    if (
      (currentPath === '/sessions' || currentPath.startsWith('/sessions/')) &&
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
        // "New Chat" should always start in the user's home directory (General).
        // getInitialWorkingDir() reflects the currently-selected project/window directory.
        await startNewSession('', setView, homeDir(), {
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

  const handleDeleteSession = React.useCallback(
    async (sessionId: string) => {
      if (!window.confirm('Delete this session? This cannot be undone.')) return;
      try {
        await deleteSession({ path: { session_id: sessionId } });
        setRecentSessions((prev) => prev.filter((s) => s.id !== sessionId));
        window.dispatchEvent(new CustomEvent(AppEvents.SESSION_DELETED, { detail: { sessionId } }));
        if (activeSessionId === sessionId) {
          navigate('/');
        }
      } catch (error) {
        console.error('Failed to delete session:', error);
      }
    },
    [activeSessionId, navigate]
  );

  const handleCloseProject = React.useCallback(
    async (projectName: string, sessionIds: string[]) => {
      const count = sessionIds.length;
      if (
        !window.confirm(
          `Close project "${projectName}"?\nThis will delete ${count} session${count !== 1 ? 's' : ''}. This cannot be undone.`
        )
      )
        return;
      try {
        await Promise.all(sessionIds.map((id) => deleteSession({ path: { session_id: id } })));
        setRecentSessions((prev) => prev.filter((s) => !sessionIds.includes(s.id)));
        sessionIds.forEach((sessionId) => {
          window.dispatchEvent(
            new CustomEvent(AppEvents.SESSION_DELETED, { detail: { sessionId } })
          );
        });
        if (activeSessionId && sessionIds.includes(activeSessionId)) {
          navigate('/');
        }
      } catch (error) {
        console.error('Failed to close project:', error);
      }
    },
    [activeSessionId, navigate]
  );

  const handleNewSessionInProject = React.useCallback(
    async (dir: string) => {
      if (dir) addRecentDir(dir);
      await startNewSession('', setView, dir || getInitialWorkingDir(), {
        allExtensions: configContext.extensionsList,
      });
    },
    [setView, addRecentDir, configContext.extensionsList]
  );

  const handleViewAllClick = React.useCallback(() => {
    navigate('/sessions');
  }, [navigate]);

  const handleOpenProjectFromDir = React.useCallback(
    async (dir: string) => {
      setProjectDropdownOpen(false);
      addRecentDir(dir);
      await startNewSession('', setView, dir, {
        allExtensions: configContext.extensionsList,
      });
    },
    [setView, addRecentDir, configContext.extensionsList]
  );

  const handleBrowseForProject = React.useCallback(async () => {
    setProjectDropdownOpen(false);
    const result = await window.electron.directoryChooser();
    if (result.canceled || !result.filePaths.length) return;
    const dir = result.filePaths[0];
    addRecentDir(dir);
    await startNewSession('', setView, dir, {
      allExtensions: configContext.extensionsList,
    });
  }, [setView, addRecentDir, configContext.extensionsList]);

  // Check if any item in a zone is currently active
  const isZoneActive = (zone: NavigationZone) => {
    return zone.items.some((item) => isActivePath(item.path));
  };

  // Filter zone items based on conditions (e.g., apps extension)
  const getVisibleItems = (zone: NavigationZone): NavigationItem[] => {
    return zone.items.filter((item) => {
      if (item.condition === 'apps') return appsExtensionEnabled;
      return true;
    });
  };

  return (
    <>
      <SidebarHeader className="px-3 py-2">
        <div className="flex items-center justify-between gap-2">
          <div className="flex items-center gap-2 group-data-[collapsible=icon]:justify-center">
            <img
              src={gooseIcon}
              alt="Goose"
              className="w-5 h-5 object-contain"
            />
            <span className="text-sm font-semibold text-text-default group-data-[collapsible=icon]:hidden">
              Projects
            </span>
          </div>

          <div className="group-data-[collapsible=icon]:hidden">
            <Popover.Root open={projectDropdownOpen} onOpenChange={setProjectDropdownOpen}>
              <Popover.Trigger asChild>
                <button
                  type="button"
                  aria-label="Projects"
                  className="p-1 rounded-md text-text-muted hover:text-text-default hover:bg-background-medium/50 transition-colors"
                >
                  <FolderPlus className="w-4 h-4" />
                </button>
              </Popover.Trigger>
              <Popover.Portal>
                <Popover.Content
                  side="right"
                  align="start"
                  sideOffset={8}
                  className="z-[60] w-56 bg-background-default border border-border-default rounded-lg shadow-lg overflow-hidden animate-in fade-in-0 zoom-in-95 data-[side=right]:slide-in-from-left-2"
                >
                  <button
                    type="button"
                    onClick={handleBrowseForProject}
                    className="w-full text-left px-3 py-2 text-sm text-text-default hover:bg-background-muted transition-colors flex items-center gap-2 border-b border-border-muted"
                  >
                    <FolderPlus className="w-4 h-4 text-text-accent" />
                    <span>Browse...</span>
                  </button>
                  {recentDirs.length > 0 && (
                    <div className="max-h-48 overflow-y-auto">
                      <div className="px-3 py-1.5 text-[10px] font-medium text-text-subtle uppercase tracking-wider">
                        Recent Projects
                      </div>
                      {recentDirs.map((dir) => (
                        <button
                          type="button"
                          key={dir}
                          onClick={() => handleOpenProjectFromDir(dir)}
                          className="w-full text-left px-3 py-1.5 text-sm text-text-muted hover:bg-background-muted hover:text-text-default transition-colors flex items-center gap-2"
                          title={dir}
                        >
                          <FolderOpen className="w-3.5 h-3.5 flex-shrink-0" />
                          <span className="truncate">{dir.split('/').pop() || dir}</span>
                        </button>
                      ))}
                    </div>
                  )}
                </Popover.Content>
              </Popover.Portal>
            </Popover.Root>
          </div>
        </div>
      </SidebarHeader>
      <SidebarContent>
        <SidebarMenu>
          {/* Sessions — General group acts as Home */}
          <SidebarGroup className="px-2">
            <SidebarGroupContent className="space-y-1">
              {/* Session list with project groups (General first) */}
              {recentSessions.length > 0 && (
                <div className="mt-1 space-y-1 max-h-[calc(100vh-280px)] overflow-y-auto group-data-[collapsible=icon]:hidden">
                  <SessionList
                    sessions={recentSessions}
                    activeSessionId={activeSessionId}
                    getSessionStatus={getSessionStatus}
                    onSessionClick={handleSessionClick}
                    onDeleteSession={handleDeleteSession}
                    onCloseProject={handleCloseProject}
                    onNewSessionInProject={handleNewSessionInProject}
                  />
                  <button type="button"
                    onClick={handleViewAllClick}
                    className="w-full text-left px-3 py-1.5 rounded-md text-sm text-text-muted hover:bg-background-medium/50 hover:text-text-default transition-colors flex items-center gap-2"
                  >
                    <History className="w-4 h-4" />
                    <span>View All</span>
                  </button>
                </div>
              )}
            </SidebarGroupContent>
          </SidebarGroup>

          <SidebarSeparator />

          {/* Navigation Zones */}
          {navigationZones.map((zone) => {
            const visibleItems = getVisibleItems(zone);
            if (visibleItems.length === 0) return null;

            const zoneActive = isZoneActive(zone);
            const ZoneIcon = zone.icon;
            const isSingleItem = visibleItems.length === 1;

            // Single-item zones render as direct links (no collapsible submenu)
            if (isSingleItem) {
              const item = visibleItems[0];
              return (
                <SidebarGroup key={zone.id} className="px-2">
                  <SidebarGroupContent className="space-y-0.5">
                    <div className="sidebar-item">
                      <SidebarMenuItem>
                        <SidebarMenuButton
                          data-testid={`sidebar-${item.label.toLowerCase()}-button`}
                          onClick={() => navigate(item.path)}
                          isActive={isActivePath(item.path)}
                          tooltip={item.tooltip}
                          className="w-full justify-start px-3 rounded-lg h-fit hover:bg-background-medium/50 transition-all duration-200 data-[active=true]:bg-background-medium"
                        >
                          <ZoneIcon className="w-4 h-4" />
                          <span>{zone.label}</span>
                        </SidebarMenuButton>
                      </SidebarMenuItem>
                    </div>
                  </SidebarGroupContent>
                </SidebarGroup>
              );
            }

            // Multi-item zones render as collapsible groups
            return (
              <SidebarGroup key={zone.id} className="px-2">
                {/* Icon-only button visible when sidebar collapsed */}
                <SidebarGroupContent className="hidden group-data-[collapsible=icon]:block">
                  <SidebarMenuItem>
                    <SidebarMenuButton
                      onClick={() =>
                        zone.route ? navigate(zone.route) : navigate(visibleItems[0].path)
                      }
                      isActive={zoneActive}
                      tooltip={zone.label}
                      className="w-full justify-start px-3 rounded-lg h-fit hover:bg-background-medium/50 transition-all duration-200 data-[active=true]:bg-background-medium"
                    >
                      <ZoneIcon className="w-4 h-4" />
                      <span>{zone.label}</span>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                </SidebarGroupContent>
                {/* Full collapsible group visible when sidebar expanded */}
                <div className="group-data-[collapsible=icon]:hidden">
                  <Collapsible defaultOpen={zoneActive}>
                    <div className="flex items-center">
                      {zone.route ? (
                        <SidebarGroupLabel
                          className="flex-1 flex items-center gap-2 px-3 py-1.5 text-xs font-medium text-text-muted uppercase tracking-wider cursor-pointer hover:text-text-default transition-colors select-none"
                          onClick={() => {
                            if (zone.route) {
                              navigate(zone.route);
                            }
                          }}
                        >
                          <ZoneIcon className="w-3.5 h-3.5" />
                          <span>{zone.label}</span>
                        </SidebarGroupLabel>
                      ) : (
                        <CollapsibleTrigger asChild>
                          <SidebarGroupLabel className="flex-1 flex items-center gap-2 px-3 py-1.5 text-xs font-medium text-text-muted uppercase tracking-wider cursor-pointer hover:text-text-default transition-colors select-none">
                            <ZoneIcon className="w-3.5 h-3.5" />
                            <span>{zone.label}</span>
                          </SidebarGroupLabel>
                        </CollapsibleTrigger>
                      )}
                      <CollapsibleTrigger asChild>
                        <button type="button" className="px-1.5 py-1.5 hover:bg-background-medium/50 rounded transition-colors">
                          <ChevronRight className="w-3 h-3 text-text-muted transition-transform duration-200 [[data-state=open]>&]:rotate-90" />
                        </button>
                      </CollapsibleTrigger>
                    </div>
                    <CollapsibleContent className="overflow-hidden transition-all data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:animate-in data-[state=open]:fade-in-0 group-data-[collapsible=icon]:hidden">
                      <SidebarGroupContent className="space-y-0.5 mt-1">
                        {visibleItems.map((item) => {
                          const ItemIcon = item.icon;
                          return (
                            <div key={item.path} className="sidebar-item">
                              <SidebarMenuItem>
                                <SidebarMenuButton
                                  data-testid={`sidebar-${item.label.toLowerCase()}-button`}
                                  onClick={() => navigate(item.path)}
                                  isActive={isActivePath(item.path)}
                                  tooltip={item.tooltip}
                                  className="w-full justify-start px-3 pl-7 rounded-lg h-fit hover:bg-background-medium/50 transition-all duration-200 data-[active=true]:bg-background-medium"
                                >
                                  <ItemIcon className="w-4 h-4" />
                                  <span>{item.label}</span>
                                </SidebarMenuButton>
                              </SidebarMenuItem>
                            </div>
                          );
                        })}
                      </SidebarGroupContent>
                    </CollapsibleContent>
                  </Collapsible>
                </div>
              </SidebarGroup>
            );
          })}
        </SidebarMenu>
      </SidebarContent>
      <SidebarFooter className="px-3 py-2 border-t border-border-muted">
        <UserAvatarMenu />
      </SidebarFooter>
    </>
  );
};

export default AppSidebar;
