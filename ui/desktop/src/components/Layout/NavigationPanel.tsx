import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useLocation } from 'react-router';
import { Archive, ChevronDown, ChevronRight, MoreHorizontal, Pencil, Trash2 } from 'lucide-react';
import { motion } from 'framer-motion';
import { toast } from 'react-toastify';
import { useNavigationContext } from './NavigationContext';
import { useConfig } from '../ConfigContext';
import { useNavigationSessions } from '../../hooks/useNavigationSessions';
import {
  isSessionBusy,
  useSessionBusy,
  useSessionBusyElsewhere,
} from '../../hooks/useSessionBusyElsewhere';
import { dispatchSessionLifecycleEvent } from '../../sessionLifecycleBridge';
import {
  NAV_ITEMS,
  SETTINGS_NAV_ITEM,
  getNavItemLabel,
  type NavItem,
} from '../../hooks/useNavigationItems';
import { AppEvents } from '../../constants/events';
import { InlineEditText } from '../common/InlineEditText';
import { SessionIndicators } from '../SessionIndicators';
import {
  acpArchiveSession,
  acpDeleteSession,
  acpRenameSession,
  type SessionListItem,
} from '../../acp/sessions';
import { Tooltip, TooltipContent, TooltipTrigger } from '../ui/Tooltip';
import { formatMessageTimestamp } from '../../utils/timeUtils';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '../ui/dropdown-menu';
import { ConfirmationModal } from '../ui/ConfirmationModal';
import { errorMessage } from '../../utils/conversionUtils';
import { cn } from '../../utils';
import type { ProjectGroup } from '../../utils/projectSessions';
import { defineMessages, useIntl } from '../../i18n';

type StreamState = 'idle' | 'loading' | 'streaming' | 'waiting' | 'error';

interface SessionStatus {
  streamState: StreamState;
  hasUnreadActivity: boolean;
}

const i18n = defineMessages({
  chats: {
    id: 'navigationPanel.chats',
    defaultMessage: 'Chats',
  },
  noChats: {
    id: 'navigationPanel.noChats',
    defaultMessage: 'No recent chats',
  },
  untitledSession: {
    id: 'navigationPanel.untitledSession',
    defaultMessage: 'Untitled session',
  },
  metaModel: {
    id: 'navigationPanel.metaModel',
    defaultMessage: 'Model',
  },
  metaDirectory: {
    id: 'navigationPanel.metaDirectory',
    defaultMessage: 'Directory',
  },
  metaStatus: {
    id: 'navigationPanel.metaStatus',
    defaultMessage: 'Status',
  },
  metaCreated: {
    id: 'navigationPanel.metaCreated',
    defaultMessage: 'Created',
  },
  metaUpdated: {
    id: 'navigationPanel.metaUpdated',
    defaultMessage: 'Updated',
  },
  statusStreaming: {
    id: 'navigationPanel.statusStreaming',
    defaultMessage: 'Streaming',
  },
  statusError: {
    id: 'navigationPanel.statusError',
    defaultMessage: 'Error',
  },
  statusUnread: {
    id: 'navigationPanel.statusUnread',
    defaultMessage: 'Unread activity',
  },
  statusIdle: {
    id: 'navigationPanel.statusIdle',
    defaultMessage: 'Idle',
  },
  sessionActions: {
    id: 'navigationPanel.sessionActions',
    defaultMessage: 'Session actions',
  },
  rename: {
    id: 'navigationPanel.rename',
    defaultMessage: 'Rename',
  },
  archive: {
    id: 'navigationPanel.archive',
    defaultMessage: 'Archive',
  },
  delete: {
    id: 'navigationPanel.delete',
    defaultMessage: 'Delete',
  },
  archiveSuccess: {
    id: 'navigationPanel.archiveSuccess',
    defaultMessage: 'Archived “{name}”',
  },
  archiveFailed: {
    id: 'navigationPanel.archiveFailed',
    defaultMessage: 'Failed to archive session: {error}',
  },
  deleteTitle: {
    id: 'navigationPanel.deleteTitle',
    defaultMessage: 'Delete chat',
  },
  deleteMessage: {
    id: 'navigationPanel.deleteMessage',
    defaultMessage: 'Are you sure you want to delete “{name}”? This cannot be undone.',
  },
  deleteConfirm: {
    id: 'navigationPanel.deleteConfirm',
    defaultMessage: 'Delete',
  },
  deleteCancel: {
    id: 'navigationPanel.deleteCancel',
    defaultMessage: 'Cancel',
  },
  deleteSuccess: {
    id: 'navigationPanel.deleteSuccess',
    defaultMessage: 'Chat deleted',
  },
  deleteFailed: {
    id: 'navigationPanel.deleteFailed',
    defaultMessage: 'Failed to delete session: {error}',
  },
});

const navItemClass = (active: boolean) =>
  cn(
    'flex flex-row items-center gap-3 outline-none no-drag w-full',
    'rounded-full px-3 py-2 text-sm font-medium transition-colors',
    active
      ? 'bg-background-tertiary text-text-primary'
      : 'text-text-primary hover:bg-background-tertiary/60'
  );

interface NavRowProps {
  item: NavItem;
  active: boolean;
  onClick: () => void;
}

const NavRow: React.FC<NavRowProps> = ({ item, active, onClick }) => {
  const intl = useIntl();
  const Icon = item.icon;
  return (
    <button onClick={onClick} className={navItemClass(active)}>
      <Icon className="w-5 h-5 flex-shrink-0 text-text-secondary" />
      <span className="text-left flex-1 truncate">{getNavItemLabel(item, intl)}</span>
      {item.getTag && (
        <span className="text-xs font-mono text-text-secondary">{item.getTag()}</span>
      )}
    </button>
  );
};

interface SessionRowProps {
  session: SessionListItem;
  active: boolean;
  status: SessionStatus | undefined;
  onClick: () => void;
  onRenamed: () => void;
  onRequestDelete: (session: SessionListItem) => void;
}

const formatTimestamp = (value?: string): string | null => {
  if (!value) return null;
  const parsed = Date.parse(value);
  if (Number.isNaN(parsed)) return null;
  return formatMessageTimestamp(parsed / 1000);
};

const MetaRow: React.FC<{ label: string; value: string }> = ({ label, value }) => (
  <div className="flex gap-2">
    <span className="text-text-inverse/60 flex-shrink-0">{label}</span>
    <span className="text-right ml-auto break-all">{value}</span>
  </div>
);

interface SessionTooltipContentProps {
  session: SessionListItem;
  statusLabel: string;
}

const SessionTooltipContent: React.FC<SessionTooltipContentProps> = ({ session, statusLabel }) => {
  const intl = useIntl();
  const model = session.modelId
    ? session.providerId
      ? `${session.modelId} (${session.providerId})`
      : session.modelId
    : session.providerId;
  const created = formatTimestamp(session.createdAt);
  const updated = formatTimestamp(session.lastMessageAt ?? session.updatedAt);

  return (
    <div className="flex flex-col gap-1 text-xs">
      <div className="font-medium break-words">
        {session.name || intl.formatMessage(i18n.untitledSession)}
      </div>
      <div className="flex flex-col gap-0.5">
        {model && <MetaRow label={intl.formatMessage(i18n.metaModel)} value={model} />}
        {session.workingDir && (
          <MetaRow label={intl.formatMessage(i18n.metaDirectory)} value={session.workingDir} />
        )}
        <MetaRow label={intl.formatMessage(i18n.metaStatus)} value={statusLabel} />
        {created && <MetaRow label={intl.formatMessage(i18n.metaCreated)} value={created} />}
        {updated && <MetaRow label={intl.formatMessage(i18n.metaUpdated)} value={updated} />}
      </div>
    </div>
  );
};

const SessionRow: React.FC<SessionRowProps> = ({
  session,
  active,
  status,
  onClick,
  onRenamed,
  onRequestDelete,
}) => {
  const intl = useIntl();
  const [isEditing, setIsEditing] = useState(false);
  const [tooltipOpen, setTooltipOpen] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);
  const [editSignal, setEditSignal] = useState(0);
  const renameRequested = useRef(false);
  const isStreaming = status?.streamState === 'streaming';
  // Archive and delete both drop the loaded agent without cancelling an active
  // run: mid-load the server can re-register the agent afterwards, and
  // streaming or waiting-on-user-input prompts would resume into a session that
  // is gone. Gate all three states — including runs owned by other windows.
  const busyElsewhere = useSessionBusyElsewhere(session.id);
  const isBusy =
    isStreaming ||
    status?.streamState === 'loading' ||
    status?.streamState === 'waiting' ||
    busyElsewhere;
  const hasError = status?.streamState === 'error';
  const hasUnread = status?.hasUnreadActivity ?? false;

  const statusLabel = isStreaming
    ? intl.formatMessage(i18n.statusStreaming)
    : hasError
      ? intl.formatMessage(i18n.statusError)
      : hasUnread
        ? intl.formatMessage(i18n.statusUnread)
        : intl.formatMessage(i18n.statusIdle);

  const handleArchive = useCallback(async () => {
    try {
      await acpArchiveSession(session.id);
      dispatchSessionLifecycleEvent(AppEvents.SESSION_ARCHIVED, {
        sessionId: session.id,
        archived: true,
      });
      toast.success(intl.formatMessage(i18n.archiveSuccess, { name: session.name }));
    } catch (error) {
      toast.error(
        intl.formatMessage(i18n.archiveFailed, { error: errorMessage(error, 'Unknown error') })
      );
    }
  }, [session.id, session.name, intl]);

  return (
    <Tooltip
      open={tooltipOpen && !isEditing && !menuOpen}
      onOpenChange={setTooltipOpen}
      delayDuration={400}
    >
      <TooltipTrigger asChild>
        <div
          onClick={() => !isEditing && onClick()}
          onContextMenu={(e) => {
            e.preventDefault();
            setMenuOpen(true);
          }}
          className={cn(
            'group flex items-center gap-2 px-3 py-1.5 rounded-full cursor-pointer text-sm',
            'hover:bg-background-tertiary/60 transition-colors',
            active && 'bg-background-tertiary'
          )}
        >
          <InlineEditText
            value={session.name}
            onSave={async (newName) => {
              await acpRenameSession(session.id, newName);
              window.dispatchEvent(
                new CustomEvent(AppEvents.SESSION_RENAMED, {
                  detail: { sessionId: session.id, newName, userInitiated: true },
                })
              );
              onRenamed();
            }}
            placeholder={intl.formatMessage(i18n.untitledSession)}
            disabled={isStreaming}
            singleClickEdit={false}
            editRequestSignal={editSignal}
            className="truncate text-text-primary flex-1 !px-0 !py-0 hover:bg-transparent"
            editClassName="!text-sm"
            onEditStart={() => setIsEditing(true)}
            onEditEnd={() => setIsEditing(false)}
          />
          <SessionIndicators isStreaming={isStreaming} hasUnread={hasUnread} hasError={hasError} />
          <DropdownMenu open={menuOpen} onOpenChange={setMenuOpen}>
            <DropdownMenuTrigger asChild>
              <button
                type="button"
                aria-label={intl.formatMessage(i18n.sessionActions)}
                onClick={(e) => e.stopPropagation()}
                className={cn(
                  'flex-shrink-0 rounded p-0.5 text-text-secondary hover:text-text-primary hover:bg-background-tertiary transition-opacity',
                  'opacity-0 group-hover:opacity-100 focus:opacity-100 data-[state=open]:opacity-100'
                )}
              >
                <MoreHorizontal className="w-4 h-4" />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent
              align="end"
              onClick={(e) => e.stopPropagation()}
              onCloseAutoFocus={(e) => {
                // Keep focus on the rename input we are about to reveal instead of
                // letting Radix return focus to the trigger.
                if (renameRequested.current) {
                  e.preventDefault();
                  renameRequested.current = false;
                }
              }}
            >
              <DropdownMenuItem
                disabled={isStreaming}
                onSelect={() => {
                  renameRequested.current = true;
                  setEditSignal((v) => v + 1);
                }}
              >
                <Pencil className="w-4 h-4" />
                {intl.formatMessage(i18n.rename)}
              </DropdownMenuItem>
              <DropdownMenuItem disabled={isBusy} onSelect={() => void handleArchive()}>
                <Archive className="w-4 h-4" />
                {intl.formatMessage(i18n.archive)}
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                variant="destructive"
                disabled={isBusy}
                onSelect={() => onRequestDelete(session)}
              >
                <Trash2 className="w-4 h-4" />
                {intl.formatMessage(i18n.delete)}
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </TooltipTrigger>
      <TooltipContent side="right" align="start" className="max-w-xs text-left">
        <SessionTooltipContent session={session} statusLabel={statusLabel} />
      </TooltipContent>
    </Tooltip>
  );
};

export const Navigation: React.FC<{ className?: string }> = ({ className }) => {
  const intl = useIntl();
  const { isNavExpanded } = useNavigationContext();
  const location = useLocation();
  const { extensionsList } = useConfig();

  const appsExtensionEnabled = !!extensionsList?.find((ext) => ext.name === 'apps')?.enabled;

  const visibleItems = useMemo<NavItem[]>(() => {
    return NAV_ITEMS.filter((item) => {
      if (item.path === '/apps') return appsExtensionEnabled;
      return true;
    });
  }, [appsExtensionEnabled]);

  const isActive = useCallback((path: string) => location.pathname === path, [location.pathname]);

  const {
    recentSessions,
    recentSessionsByProject,
    activeSessionId,
    fetchSessions,
    handleNavClick,
    handleSessionClick,
  } = useNavigationSessions();

  const [sessionStatuses, setSessionStatuses] = useState<Map<string, SessionStatus>>(new Map());

  useEffect(() => {
    const handleStatusUpdate = (event: Event) => {
      const { sessionId, streamState } = (event as CustomEvent).detail;
      setSessionStatuses((prev) => {
        const existing = prev.get(sessionId);
        const shouldMarkUnread =
          (existing?.streamState === 'streaming' || existing?.streamState === 'waiting') &&
          streamState === 'idle';
        const next = new Map(prev);
        next.set(sessionId, {
          streamState,
          hasUnreadActivity: existing?.hasUnreadActivity || shouldMarkUnread,
        });
        return next;
      });
    };

    window.addEventListener(AppEvents.SESSION_STATUS_UPDATE, handleStatusUpdate);
    return () => window.removeEventListener(AppEvents.SESSION_STATUS_UPDATE, handleStatusUpdate);
  }, []);

  const clearUnread = useCallback((sessionId: string) => {
    setSessionStatuses((prev) => {
      const status = prev.get(sessionId);
      if (status?.hasUnreadActivity) {
        const next = new Map(prev);
        next.set(sessionId, { ...status, hasUnreadActivity: false });
        return next;
      }
      return prev;
    });
  }, []);

  const navFocusRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (isNavExpanded) {
      fetchSessions();
      requestAnimationFrame(() => navFocusRef.current?.focus());
    }
  }, [isNavExpanded, fetchSessions]);

  const [isChatsExpanded, setIsChatsExpanded] = useState(true);
  const [collapsedProjects, setCollapsedProjects] = useState<Set<string>>(new Set());

  const toggleProjectCollapsed = useCallback((path: string) => {
    setCollapsedProjects((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  }, []);

  const [sessionPendingDelete, setSessionPendingDelete] = useState<SessionListItem | null>(null);

  const handleRequestDelete = useCallback((session: SessionListItem) => {
    setSessionPendingDelete(session);
  }, []);

  // The dialog can sit open while the session starts a run elsewhere, so
  // re-check instead of trusting the state the menu item was disabled on.
  const pendingDeleteBusy = useSessionBusy(sessionPendingDelete?.id ?? '');

  const handleConfirmDelete = useCallback(async () => {
    if (!sessionPendingDelete) return;
    const { id, name } = sessionPendingDelete;
    if (isSessionBusy(id)) return;
    setSessionPendingDelete(null);
    try {
      await acpDeleteSession(id);
      // App's SESSION_DELETED handler runs the ACP cleanup for every window.
      dispatchSessionLifecycleEvent(AppEvents.SESSION_DELETED, { sessionId: id });
      toast.success(intl.formatMessage(i18n.deleteSuccess));
    } catch (error) {
      toast.error(
        intl.formatMessage(i18n.deleteFailed, {
          name,
          error: errorMessage(error, 'Unknown error'),
        })
      );
    }
  }, [sessionPendingDelete, intl]);

  if (!isNavExpanded) return null;

  return (
    <motion.div
      ref={navFocusRef}
      tabIndex={-1}
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={{ duration: 0.15 }}
      className={cn('bg-background-primary outline-none flex flex-col h-full', className)}
    >
      <div className="h-[48px] no-drag" />

      <div className="px-2 flex flex-col gap-0.5">
        {visibleItems.map((item) => (
          <NavRow
            key={item.id}
            item={item}
            active={isActive(item.path)}
            onClick={() => handleNavClick(item.path)}
          />
        ))}
      </div>

      <div className="flex-1 min-h-0 flex flex-col mt-3">
        <button
          onClick={() => setIsChatsExpanded((v) => !v)}
          className="flex items-center gap-1 px-4 py-1 text-xs font-semibold uppercase tracking-wider text-text-secondary hover:text-text-primary transition-colors self-start"
        >
          {isChatsExpanded ? (
            <ChevronDown className="w-3 h-3" />
          ) : (
            <ChevronRight className="w-3 h-3" />
          )}
          <span>{intl.formatMessage(i18n.chats)}</span>
        </button>
        {isChatsExpanded && (
          <div className="flex-1 min-h-0 overflow-y-auto px-2 pb-2 mt-1">
            {recentSessions.length === 0 ? (
              <div className="px-3 py-2 text-xs text-text-secondary">
                {intl.formatMessage(i18n.noChats)}
              </div>
            ) : recentSessionsByProject.length > 1 ? (
              recentSessionsByProject.map((group: ProjectGroup) => {
                const isCollapsed = collapsedProjects.has(group.path);
                return (
                  <React.Fragment key={group.path}>
                    <button
                      onClick={() => toggleProjectCollapsed(group.path)}
                      aria-expanded={!isCollapsed}
                      className="flex items-center gap-1 w-full px-3 pt-2 pb-0.5 text-[10px] uppercase tracking-wider text-text-tertiary hover:text-text-secondary transition-colors"
                      title={group.path}
                    >
                      {isCollapsed ? (
                        <ChevronRight className="w-3 h-3 flex-shrink-0" />
                      ) : (
                        <ChevronDown className="w-3 h-3 flex-shrink-0" />
                      )}
                      <span className="truncate">{group.label}</span>
                    </button>
                    {!isCollapsed &&
                      group.sessions.map((session) => (
                        <SessionRow
                          key={session.id}
                          session={session}
                          active={session.id === activeSessionId}
                          status={sessionStatuses.get(session.id)}
                          onClick={() => {
                            clearUnread(session.id);
                            handleSessionClick(session.id);
                          }}
                          onRenamed={fetchSessions}
                          onRequestDelete={handleRequestDelete}
                        />
                      ))}
                  </React.Fragment>
                );
              })
            ) : (
              recentSessions.map((session) => (
                <SessionRow
                  key={session.id}
                  session={session}
                  active={session.id === activeSessionId}
                  status={sessionStatuses.get(session.id)}
                  onClick={() => {
                    clearUnread(session.id);
                    handleSessionClick(session.id);
                  }}
                  onRenamed={fetchSessions}
                  onRequestDelete={handleRequestDelete}
                />
              ))
            )}
          </div>
        )}
      </div>

      <div className="px-2 pt-2 pb-2 border-t border-border-secondary">
        <NavRow
          item={SETTINGS_NAV_ITEM}
          active={isActive(SETTINGS_NAV_ITEM.path)}
          onClick={() => handleNavClick(SETTINGS_NAV_ITEM.path)}
        />
      </div>

      <ConfirmationModal
        isOpen={sessionPendingDelete !== null}
        title={intl.formatMessage(i18n.deleteTitle)}
        message={intl.formatMessage(i18n.deleteMessage, { name: sessionPendingDelete?.name ?? '' })}
        confirmLabel={intl.formatMessage(i18n.deleteConfirm)}
        cancelLabel={intl.formatMessage(i18n.deleteCancel)}
        confirmVariant="destructive"
        confirmDisabled={pendingDeleteBusy}
        onConfirm={handleConfirmDelete}
        onCancel={() => setSessionPendingDelete(null)}
      />
    </motion.div>
  );
};
