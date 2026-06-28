import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useLocation } from 'react-router-dom';
import { ChevronDown, ChevronRight, Trash2 } from 'lucide-react';
import { motion } from 'framer-motion';
import { toast } from 'react-toastify';
import { useNavigationContext } from './NavigationContext';
import { useConfig } from '../ConfigContext';
import { useNavigationSessions } from '../../hooks/useNavigationSessions';
import {
  NAV_ITEMS,
  SETTINGS_NAV_ITEM,
  getNavItemLabel,
  type NavItem,
} from '../../hooks/useNavigationItems';
import { AppEvents } from '../../constants/events';
import { InlineEditText } from '../common/InlineEditText';
import { SessionIndicators } from '../SessionIndicators';
import { ConfirmationModal } from '../ui/ConfirmationModal';
import { acpDeleteSession, acpRenameSession, type SessionListItem } from '../../acp/sessions';
import { acpChatSessionActions } from '../../acp/chatSessionStore';
import { cancelAcpPermissionRequestsForSession } from '../../acp/permissionRequests';
import { cancelAcpElicitationRequestsForSession } from '../../acp/elicitationRequests';
import { errorMessage } from '../../utils/conversionUtils';
import { cn } from '../../utils';
import { defineMessages, useIntl } from '../../i18n';

type StreamState = 'idle' | 'loading' | 'streaming' | 'error';

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
  // Reuse the existing Session History strings so no new translations are needed.
  deleteSession: { id: 'sessions.action.delete', defaultMessage: 'Delete session' },
  deleteTitle: { id: 'sessions.delete.title', defaultMessage: 'Delete Session' },
  deleteMessage: {
    id: 'sessions.delete.message',
    defaultMessage:
      'Are you sure you want to delete the session "{name}"? This action cannot be undone.',
  },
  cancel: { id: 'sessions.cancel', defaultMessage: 'Cancel' },
  deleteSuccess: { id: 'sessions.toast.deleted', defaultMessage: 'Session deleted successfully' },
  deleteFailed: {
    id: 'sessions.toast.deleteFailed',
    defaultMessage: 'Failed to delete session "{name}": {error}',
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
  onDelete: () => void;
}

const SessionRow: React.FC<SessionRowProps> = ({
  session,
  active,
  status,
  onClick,
  onRenamed,
  onDelete,
}) => {
  const intl = useIntl();
  const [isEditing, setIsEditing] = useState(false);
  const isStreaming = status?.streamState === 'streaming';
  const hasError = status?.streamState === 'error';
  const hasUnread = status?.hasUnreadActivity ?? false;

  return (
    <div
      onClick={() => !isEditing && onClick()}
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
        className="truncate text-text-primary flex-1 !px-0 !py-0 hover:bg-transparent"
        editClassName="!text-sm"
        onEditStart={() => setIsEditing(true)}
        onEditEnd={() => setIsEditing(false)}
      />
      <SessionIndicators isStreaming={isStreaming} hasUnread={hasUnread} hasError={hasError} />
      {!isEditing && (
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onDelete();
          }}
          title={intl.formatMessage(i18n.deleteSession)}
          aria-label={intl.formatMessage(i18n.deleteSession)}
          className="shrink-0 p-1 rounded opacity-0 group-hover:opacity-100 focus:opacity-100 hover:bg-red-50 dark:hover:bg-red-900/20 transition-opacity"
        >
          <Trash2 className="w-3.5 h-3.5 text-text-secondary hover:text-red-600" />
        </button>
      )}
    </div>
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

  const { recentSessions, activeSessionId, fetchSessions, handleNavClick, handleSessionClick } =
    useNavigationSessions();

  const [sessionStatuses, setSessionStatuses] = useState<Map<string, SessionStatus>>(new Map());
  const [sessionToDelete, setSessionToDelete] = useState<SessionListItem | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);

  const handleConfirmDelete = useCallback(async () => {
    if (!sessionToDelete) return;
    const { id, name } = sessionToDelete;
    setIsDeleting(true);
    try {
      await acpDeleteSession(id);
      window.dispatchEvent(
        new CustomEvent(AppEvents.SESSION_DELETED, { detail: { sessionId: id } })
      );
      cancelAcpPermissionRequestsForSession(id);
      cancelAcpElicitationRequestsForSession(id);
      acpChatSessionActions.deleteSnapshot(id);
      toast.success(intl.formatMessage(i18n.deleteSuccess));
      fetchSessions();
    } catch (error) {
      toast.error(
        intl.formatMessage(i18n.deleteFailed, {
          name,
          error: errorMessage(error, 'Unknown error'),
        })
      );
    } finally {
      setIsDeleting(false);
      setSessionToDelete(null);
    }
  }, [sessionToDelete, intl, fetchSessions]);

  useEffect(() => {
    const handleStatusUpdate = (event: Event) => {
      const { sessionId, streamState } = (event as CustomEvent).detail;
      setSessionStatuses((prev) => {
        const existing = prev.get(sessionId);
        const shouldMarkUnread = existing?.streamState === 'streaming' && streamState === 'idle';
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

      {/* Nav items */}
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

      {/* Chats section — takes remaining vertical space */}
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
                  onDelete={() => setSessionToDelete(session)}
                />
              ))
            )}
          </div>
        )}
      </div>

      {/* Settings pinned to bottom */}
      <div className="px-2 pt-2 pb-2 border-t border-border-secondary">
        <NavRow
          item={SETTINGS_NAV_ITEM}
          active={isActive(SETTINGS_NAV_ITEM.path)}
          onClick={() => handleNavClick(SETTINGS_NAV_ITEM.path)}
        />
      </div>

      <ConfirmationModal
        isOpen={sessionToDelete !== null}
        title={intl.formatMessage(i18n.deleteTitle)}
        message={intl.formatMessage(i18n.deleteMessage, { name: sessionToDelete?.name ?? '' })}
        confirmLabel={intl.formatMessage(i18n.deleteTitle)}
        cancelLabel={intl.formatMessage(i18n.cancel)}
        confirmVariant="destructive"
        isSubmitting={isDeleting}
        onConfirm={handleConfirmDelete}
        onCancel={() => setSessionToDelete(null)}
      />
    </motion.div>
  );
};
