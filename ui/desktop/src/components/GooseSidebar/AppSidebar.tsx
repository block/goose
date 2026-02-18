import { AppEvents } from '../../constants/events';
import React, { useEffect, useState, useRef } from 'react';
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
  X,
  Workflow,
} from 'lucide-react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import {
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarSeparator,
  SidebarTrigger,
  useSidebar,
} from '../ui/sidebar';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '../ui/collapsible';
import { Gear } from '../icons';
import { View, ViewOptions } from '../../utils/navigationUtils';
import { DEFAULT_CHAT_TITLE, useChatContext } from '../../contexts/ChatContext';
import { deleteSession, listSessions, Session, updateSessionName } from '../../api';
import { resumeSession, startNewSession, shouldShowNewChatTitle } from '../../sessions';
import { useNavigation } from '../../hooks/useNavigation';
import { SessionIndicators } from '../SessionIndicators';
import { useSidebarSessionStatus } from '../../hooks/useSidebarSessionStatus';
import { getInitialWorkingDir } from '../../utils/workingDir';
import { useConfig } from '../ConfigContext';
import { InlineEditText } from '../common/InlineEditText';
import { useProjectPreferences } from '../../hooks/useProjectPreferences';

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
    if (cleanPath !== iwdClean && cleanPath.startsWith(iwdClean + '/')) {
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
      <button
        onClick={() => onSessionClick(session)}
        className={`w-full text-left ml-3 px-1.5 py-1.5 pr-7 rounded-md text-sm transition-colors flex items-center gap-1 min-w-0 ${
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
        <SessionIndicators isStreaming={isStreaming} hasUnread={hasUnread} hasError={hasError} />
      </button>
      {onDeleteSession && !isStreaming && (
        <button
          onClick={handleDelete}
          className="absolute right-1 top-1/2 -translate-y-1/2 opacity-0 group-hover/session:opacity-100 p-1 hover:bg-background-danger-muted rounded transition-all"
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
          const ProjectIcon = isGeneral ? Home : FolderOpen;

          return (
            <div key={group.project}>
              <div className="flex items-center group/project">
                <button
                  onClick={() => toggleCollapsed(group.project)}
                  className="flex items-center gap-1.5 flex-1 min-w-0 px-2 py-1 text-xs font-medium text-text-muted hover:text-text-default transition-colors rounded-md hover:bg-background-medium/30"
                >
                  <ProjectIcon className="w-3 h-3 flex-shrink-0" />
                  <span className="truncate">{group.project}</span>
                  {pinned && <Pin className="w-2.5 h-2.5 flex-shrink-0 text-text-accent" />}
                  <span className="text-[10px] opacity-60 ml-auto flex-shrink-0">
                    {group.sessions.length}
                  </span>
                  <ChevronRight
                    className={`w-3 h-3 flex-shrink-0 transition-transform duration-200 ${!collapsed ? 'rotate-90' : ''}`}
                  />
                </button>
                {onNewSessionInProject && (
                  <button
                    onClick={() => {
                      const dir = isGeneral
                        ? ''
                        : group.sessions[0]?.working_dir || '';
                      onNewSessionInProject(dir);
                    }}
                    className="opacity-0 group-hover/project:opacity-100 p-1 hover:bg-background-muted rounded transition-all"
                    title={isGeneral ? 'New session' : `New session in ${group.project}`}
                  >
                    <Plus className="w-3 h-3 text-text-muted hover:text-text-default" />
                  </button>
                )}
                {!isGeneral && (
                  <button
                    onClick={() => togglePin(group.project)}
                    className="opacity-0 group-hover/project:opacity-100 p-1 hover:bg-background-medium/50 rounded transition-all"
                    title={pinned ? 'Unpin project' : 'Pin project'}
                  >
                    {pinned ? (
                      <PinOff className="w-3 h-3 text-text-muted" />
                    ) : (
                      <Pin className="w-3 h-3 text-text-muted" />
                    )}
                  </button>
                )}
                {!isGeneral && (
                  <button
                    onClick={() =>
                      onCloseProject(
                        group.project,
                        group.sessions.map((s) => s.id)
                      )
                    }
                    className="opacity-0 group-hover/project:opacity-100 p-1 hover:bg-background-danger-muted rounded transition-all"
                    title={`Close project "${group.project}" and delete ${group.sessions.length} session${group.sessions.length !== 1 ? 's' : ''}`}
                  >
                    <X className="w-3 h-3 text-text-muted hover:text-text-danger" />
                  </button>
                )}
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
  const [searchParams] = useSearchParams();
  const [recentSessions, setRecentSessions] = useState<Session[]>([]);
  const [isChatExpanded, setIsChatExpanded] = useState(true);
  const activeSessionId = searchParams.get('resumeSessionId') ?? undefined;
  const { getSessionStatus, clearUnread } = useSidebarSessionStatus(activeSessionId);
  const { addRecentDir, recentDirs } = useProjectPreferences();
  const [projectDropdownOpen, setProjectDropdownOpen] = useState(false);
  const projectDropdownRef = useRef<HTMLDivElement>(null);
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

  // Find current navigation item across all zones for window title
  useEffect(() => {
    const allItems = navigationZones.flatMap((z) => z.items);
    const currentItem = allItems.find((item) => item.path === currentPath);

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

  useEffect(() => {
    if (!projectDropdownOpen) return;
    const handleClickOutside = (e: MouseEvent) => {
      if (projectDropdownRef.current && !projectDropdownRef.current.contains(e.target as Node)) {
        setProjectDropdownOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [projectDropdownOpen]);

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

  const { state: sidebarState } = useSidebar();
  const isCollapsedSidebar = sidebarState === 'collapsed';

  return (
    <>
      <SidebarHeader className="flex flex-row items-center gap-1 px-2 py-2">
        <SidebarTrigger className="hover:bg-background-medium/50" />
        {!isCollapsedSidebar && (
          <span className="text-sm font-semibold text-text-default truncate flex-1">Goose</span>
        )}
      </SidebarHeader>
      <SidebarContent>
        <SidebarMenu>
          {/* Home + Chat Zone */}
          <SidebarGroup className="px-2">
            <SidebarGroupContent className="space-y-1">
              {/* Home (unified — shows chat with WelcomeState when no messages) */}
              <Collapsible open={isChatExpanded} onOpenChange={setIsChatExpanded}>
                <div className="sidebar-item">
                  <SidebarMenuItem>
                    <div className="flex items-center w-full">
                      <SidebarMenuButton
                        data-testid="sidebar-home-button"
                        onClick={handleNewChat}
                        isActive={isActivePath('/pair') || isActivePath('/')}
                        tooltip="Home — Start a new chat"
                        className="flex-1 justify-start px-3 rounded-lg h-fit hover:bg-background-medium/50 transition-all duration-200 data-[active=true]:bg-background-medium"
                      >
                        <Home className="w-4 h-4" />
                        <span>Home</span>
                      </SidebarMenuButton>
                      <div className="relative" ref={projectDropdownRef}>
                        <button
                          onClick={() => setProjectDropdownOpen((prev) => !prev)}
                          className="flex items-center justify-center w-6 h-8 hover:bg-background-medium/50 rounded-md transition-colors"
                          aria-label="Open project"
                          title="Open project in new session"
                        >
                          <FolderPlus className="w-3.5 h-3.5 text-text-muted" />
                        </button>
                        {projectDropdownOpen && (
                          <div className="absolute left-0 top-full mt-1 w-56 z-50 bg-background-default border border-border-default rounded-lg shadow-lg overflow-hidden">
                            <button
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
                          </div>
                        )}
                      </div>
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
                    <div className="mt-1 space-y-1 max-h-[calc(100vh-320px)] overflow-y-auto">
                      <SessionList
                        sessions={recentSessions}
                        activeSessionId={activeSessionId}
                        getSessionStatus={getSessionStatus}
                        onSessionClick={handleSessionClick}
                        onDeleteSession={handleDeleteSession}
                        onCloseProject={handleCloseProject}
                        onNewSessionInProject={handleNewSessionInProject}
                      />
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
                <Collapsible defaultOpen={zoneActive}>
                  <div className="flex items-center">
                    {zone.route ? (
                      <SidebarGroupLabel
                        className="flex-1 flex items-center gap-2 px-3 py-1.5 text-xs font-medium text-text-muted uppercase tracking-wider cursor-pointer hover:text-text-default transition-colors select-none"
                        onClick={() => navigate(zone.route!)}
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
                      <button className="px-1.5 py-1.5 hover:bg-background-medium/50 rounded transition-colors">
                        <ChevronRight className="w-3 h-3 text-text-muted transition-transform duration-200 [[data-state=open]>&]:rotate-90" />
                      </button>
                    </CollapsibleTrigger>
                  </div>
                  <CollapsibleContent className="overflow-hidden transition-all data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:animate-in data-[state=open]:fade-in-0">
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
              </SidebarGroup>
            );
          })}
        </SidebarMenu>
      </SidebarContent>
    </>
  );
};

export default AppSidebar;
