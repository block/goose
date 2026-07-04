import React, { startTransition, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { defineMessages, useIntl } from '../../i18n';
import {
  Archive,
  Calendar,
  Copy,
  Download,
  Edit2,
  ExternalLink,
  Folder,
  LoaderCircle,
  MessageSquareText,
  RotateCcw,
  Share2,
  Trash2,
} from 'lucide-react';
import { toast } from 'react-toastify';
import { Card } from '../ui/card';
import { Button } from '../ui/button';
import { ScrollArea } from '../ui/scroll-area';
import { SearchView } from '../conversation/SearchView';
import { Skeleton } from '../ui/skeleton';
import { ConfirmationModal } from '../ui/ConfirmationModal';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '../ui/dialog';
import { formatMessageTimestamp } from '../../utils/timeUtils';
import { errorMessage } from '../../utils/conversionUtils';
import { groupSessionsByDate, sessionActivityAt, type DateGroup } from '../../utils/dateUtils';
import {
  acpDeleteSession,
  acpExportSession,
  acpForkSession,
  acpListSessions,
  acpRenameSession,
  acpShareSessionNostr,
  acpUnarchiveSession,
  type SessionListItem,
} from '../../acp/sessions';
import { AppEvents } from '../../constants/events';
import { acpChatSessionActions } from '../../acp/chatSessionStore';
import { cancelAcpElicitationRequestsForSession } from '../../acp/elicitationRequests';
import { cancelAcpPermissionRequestsForSession } from '../../acp/permissionRequests';
import {
  ArchiveFolderNotConfiguredError,
  archiveSessionToConfiguredFolder,
  getArchiveFolder,
  getTrackedArchiveFile,
  removeTrackedArchiveFile,
} from '../../sessionArchive';

const i18n = defineMessages({
  editSessionTitle: { id: 'sessions.edit.title', defaultMessage: 'Edit Session Description' },
  editSessionPlaceholder: {
    id: 'sessions.edit.placeholder',
    defaultMessage: 'Enter session description',
  },
  cancel: { id: 'sessions.cancel', defaultMessage: 'Cancel' },
  save: { id: 'sessions.save', defaultMessage: 'Save' },
  saving: { id: 'sessions.saving', defaultMessage: 'Saving...' },
  sessionUpdated: {
    id: 'sessions.toast.updated',
    defaultMessage: 'Session description updated successfully',
  },
  sessionUpdateFailed: {
    id: 'sessions.toast.updateFailed',
    defaultMessage: 'Failed to update session description: {error}',
  },
  searchPlaceholder: { id: 'sessions.searchPlaceholder', defaultMessage: 'Search history...' },
  searchArchivesPlaceholder: {
    id: 'sessions.search.archivedPlaceholder',
    defaultMessage: 'Search archived sessions...',
  },
  errorLoading: { id: 'sessions.error.loading', defaultMessage: 'Error Loading Sessions' },
  tryAgain: { id: 'sessions.error.tryAgain', defaultMessage: 'Try Again' },
  noSessions: { id: 'sessions.empty.title', defaultMessage: 'No chat sessions found' },
  noSessionsDesc: {
    id: 'sessions.empty.description',
    defaultMessage: 'Your chat history will appear here',
  },
  noArchivedSessions: {
    id: 'sessions.archived.empty.title',
    defaultMessage: 'No archived sessions found',
  },
  noArchivedSessionsDesc: {
    id: 'sessions.archived.empty.description',
    defaultMessage: 'Archived sessions will appear here once you archive them.',
  },
  noMatching: { id: 'sessions.search.noResults', defaultMessage: 'No matching sessions found' },
  noMatchingDesc: {
    id: 'sessions.search.noResultsDesc',
    defaultMessage: 'Try adjusting your search terms',
  },
  noMatchingArchives: {
    id: 'sessions.archived.search.noResults',
    defaultMessage: 'No matching archived sessions found',
  },
  loadingMore: { id: 'sessions.loadingMore', defaultMessage: 'Loading more sessions...' },
  deleteTitle: { id: 'sessions.delete.title', defaultMessage: 'Delete Session' },
  deleteMessage: {
    id: 'sessions.delete.message',
    defaultMessage:
      'Are you sure you want to delete the session "{name}"? This action cannot be undone.',
  },
  deleteWithArchiveFileMessage: {
    id: 'sessions.delete.withArchiveFile',
    defaultMessage:
      'Delete the session "{name}" permanently? Its tracked archive file will also be removed from disk.',
  },
  deleteTrackedFileDetail: {
    id: 'sessions.delete.trackedFileDetail',
    defaultMessage: 'Tracked archive file: {filePath}',
  },
  duplicateSuccess: {
    id: 'sessions.toast.duplicated',
    defaultMessage: 'Session "{name}" duplicated successfully',
  },
  duplicateFailed: {
    id: 'sessions.toast.duplicateFailed',
    defaultMessage: 'Failed to duplicate session: {error}',
  },
  deleteSuccess: {
    id: 'sessions.toast.deleted',
    defaultMessage: 'Session deleted successfully',
  },
  deleteFailed: {
    id: 'sessions.toast.deleteFailed',
    defaultMessage: 'Failed to delete session "{name}": {error}',
  },
  deleteArchiveFileFailed: {
    id: 'sessions.toast.deleteArchiveFileFailed',
    defaultMessage: 'Deleted session, but failed to remove archive file on disk.',
  },
  exportSuccess: {
    id: 'sessions.toast.exported',
    defaultMessage: 'Session exported successfully',
  },
  shareNostrSuccess: {
    id: 'sessions.toast.shareNostr',
    defaultMessage: 'Encrypted Nostr share link created',
  },
  shareNostrFailed: {
    id: 'sessions.toast.shareNostrFailed',
    defaultMessage: 'Failed to create Nostr share link: {error}',
  },
  copied: { id: 'sessions.toast.copied', defaultMessage: 'Copied to clipboard' },
  archiveSession: { id: 'sessions.action.archive', defaultMessage: 'Archive session' },
  archiveTitle: { id: 'sessions.archive.title', defaultMessage: 'Archive Session' },
  archiveMessage: {
    id: 'sessions.archive.message',
    defaultMessage:
      'Archive "{name}" to the configured folder and hide it from active session history?',
  },
  archiveDetail: {
    id: 'sessions.archive.detail',
    defaultMessage: 'Archive folder: {folder}. You can restore this session from Archived.',
  },
  archiveSuccess: {
    id: 'sessions.toast.archived',
    defaultMessage: 'Session archived successfully',
  },
  archiveFailed: {
    id: 'sessions.toast.archiveFailed',
    defaultMessage: 'Failed to archive session "{name}": {error}',
  },
  archiveFolderMissing: {
    id: 'sessions.toast.archiveFolderMissing',
    defaultMessage: 'Configure an archive folder in App Settings before archiving sessions.',
  },
  restoreSession: { id: 'sessions.action.restore', defaultMessage: 'Restore session' },
  restoreTitle: { id: 'sessions.restore.title', defaultMessage: 'Restore Session' },
  restoreMessage: {
    id: 'sessions.restore.message',
    defaultMessage: 'Restore "{name}" back into active session history?',
  },
  restoreSuccess: {
    id: 'sessions.toast.restored',
    defaultMessage: 'Session restored successfully',
  },
  restoreFailed: {
    id: 'sessions.toast.restoreFailed',
    defaultMessage: 'Failed to restore session "{name}": {error}',
  },
  openInNewWindow: {
    id: 'sessions.action.openNewWindow',
    defaultMessage: 'Open in new window',
  },
  editSessionName: { id: 'sessions.action.editName', defaultMessage: 'Edit session name' },
  duplicateSession: { id: 'sessions.action.duplicate', defaultMessage: 'Duplicate session' },
  deleteSession: { id: 'sessions.action.delete', defaultMessage: 'Delete session' },
  exportSession: { id: 'sessions.action.export', defaultMessage: 'Export session' },
  shareNostrSession: {
    id: 'sessions.action.shareNostr',
    defaultMessage: 'Share encrypted Nostr link',
  },
  shareNostrTitle: {
    id: 'sessions.shareNostr.title',
    defaultMessage: 'Encrypted Nostr Share Link',
  },
  shareNostrDesc: {
    id: 'sessions.shareNostr.description',
    defaultMessage:
      'Anyone with this link can fetch and decrypt the session. Treat it like a secret.',
  },
  close: { id: 'sessions.close', defaultMessage: 'Close' },
  archivedSnippetFallback: {
    id: 'sessions.archived.snippetFallback',
    defaultMessage: 'No message preview available',
  },
  archivedAtLabel: { id: 'sessions.archived.at', defaultMessage: 'Archived' },
});

interface EditSessionModalProps {
  session: SessionListItem | null;
  isOpen: boolean;
  onClose: () => void;
  onSave: (sessionId: string, newDescription: string) => Promise<void>;
  disabled?: boolean;
}

const EditSessionModal = React.memo<EditSessionModalProps>(
  ({ session, isOpen, onClose, onSave, disabled = false }) => {
    const intl = useIntl();
    const [description, setDescription] = useState('');
    const [isUpdating, setIsUpdating] = useState(false);

    useEffect(() => {
      if (session && isOpen) {
        setDescription(session.name);
      } else if (!isOpen) {
        setDescription('');
        setIsUpdating(false);
      }
    }, [session, isOpen]);

    const handleSave = useCallback(async () => {
      if (!session || disabled) return;

      const trimmedDescription = description.trim();
      if (trimmedDescription === session.name) {
        onClose();
        return;
      }

      setIsUpdating(true);
      try {
        await acpRenameSession(session.id, trimmedDescription);
        await onSave(session.id, trimmedDescription);
        onClose();
        setTimeout(() => {
          toast.success(intl.formatMessage(i18n.sessionUpdated));
        }, 300);
      } catch (error) {
        const errMsg = errorMessage(error, 'Unknown error occurred');
        toast.error(intl.formatMessage(i18n.sessionUpdateFailed, { error: errMsg }));
        setDescription(session.name);
      } finally {
        setIsUpdating(false);
      }
    }, [description, disabled, intl, onClose, onSave, session]);

    const handleCancel = useCallback(() => {
      if (!isUpdating) {
        onClose();
      }
    }, [isUpdating, onClose]);

    const handleKeyDown = useCallback(
      (event: React.KeyboardEvent<HTMLInputElement>) => {
        if (event.key === 'Enter' && !isUpdating) {
          void handleSave();
        } else if (event.key === 'Escape' && !isUpdating) {
          handleCancel();
        }
      },
      [handleCancel, handleSave, isUpdating]
    );

    if (!isOpen || !session) {
      return null;
    }

    return (
      <div className="fixed inset-0 z-[300] flex items-center justify-center bg-black/50">
        <div className="w-[500px] max-w-[90vw] rounded-lg border border-border-primary bg-background-primary p-6">
          <h3 className="mb-4 text-lg font-medium text-text-primary">
            {intl.formatMessage(i18n.editSessionTitle)}
          </h3>

          <input
            id="session-description"
            type="text"
            value={description}
            onChange={(event) => setDescription(event.target.value)}
            className="w-full rounded-lg border border-border-primary bg-background-primary p-3 text-text-primary focus:outline-none focus:ring-2 focus:ring-blue-500"
            placeholder={intl.formatMessage(i18n.editSessionPlaceholder)}
            autoFocus
            maxLength={200}
            onKeyDown={handleKeyDown}
            disabled={isUpdating || disabled}
          />

          <div className="mt-6 flex justify-end space-x-3">
            <Button onClick={handleCancel} variant="ghost" disabled={isUpdating || disabled}>
              {intl.formatMessage(i18n.cancel)}
            </Button>
            <Button
              onClick={() => void handleSave()}
              disabled={!description.trim() || isUpdating || disabled}
              variant="default"
            >
              {isUpdating ? intl.formatMessage(i18n.saving) : intl.formatMessage(i18n.save)}
            </Button>
          </div>
        </div>
      </div>
    );
  }
);

EditSessionModal.displayName = 'EditSessionModal';

function useDebounce<T>(value: T, delay: number): T {
  const [debouncedValue, setDebouncedValue] = useState<T>(value);

  useEffect(() => {
    const handler = setTimeout(() => {
      setDebouncedValue(value);
    }, delay);

    return () => {
      window.clearTimeout(handler);
    };
  }, [delay, value]);

  return debouncedValue;
}

type SessionPaneMode = 'active' | 'archived';

interface SessionListPaneProps {
  mode: SessionPaneMode;
  onSelectSession?: (sessionId: string) => void;
}

interface DeleteDialogState {
  filePath?: string;
  hasTrackedArchiveFile: boolean;
  session: SessionListItem;
}

function archiveTimestamp(session: SessionListItem): string {
  return session.archivedAt ?? session.updatedAt;
}

export default function SessionListPane({ mode, onSelectSession }: SessionListPaneProps) {
  const intl = useIntl();
  const navigate = useNavigate();
  const [sessions, setSessions] = useState<SessionListItem[]>([]);
  const [isPrefetchingSessions, setIsPrefetchingSessions] = useState(false);
  const [dateGroups, setDateGroups] = useState<DateGroup[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [showSkeleton, setShowSkeleton] = useState(true);
  const [showContent, setShowContent] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [visibleGroupsCount, setVisibleGroupsCount] = useState(15);
  const [showEditModal, setShowEditModal] = useState(false);
  const [editingSession, setEditingSession] = useState<SessionListItem | null>(null);
  const [deleteDialogState, setDeleteDialogState] = useState<DeleteDialogState | null>(null);
  const [archiveSession, setArchiveSession] = useState<SessionListItem | null>(null);
  const [archiveFolder, setArchiveFolder] = useState<string | null>(null);
  const [restoreSession, setRestoreSession] = useState<SessionListItem | null>(null);
  const [showShareLinkModal, setShowShareLinkModal] = useState(false);
  const [shareLink, setShareLink] = useState('');
  const [sharingSessionId, setSharingSessionId] = useState<string | null>(null);
  const [nostrEnabled, setNostrEnabled] = useState(true);
  const [searchTerm, setSearchTerm] = useState('');
  const debouncedSearchTerm = useDebounce(searchTerm, 300);
  const debouncedSearchTermRef = useRef(debouncedSearchTerm);
  const loadGenerationRef = useRef(0);
  const hasLoadedRef = useRef(false);
  debouncedSearchTermRef.current = debouncedSearchTerm;

  const visibleDateGroups = useMemo(() => {
    return dateGroups.slice(0, visibleGroupsCount);
  }, [dateGroups, visibleGroupsCount]);

  const previousSearchTermRef = useRef('');
  useEffect(() => {
    const wasSearching = previousSearchTermRef.current.length > 0;
    const isSearching = debouncedSearchTerm.length > 0;
    previousSearchTermRef.current = debouncedSearchTerm;

    if (isSearching) {
      setVisibleGroupsCount(dateGroups.length);
    } else if (wasSearching) {
      setVisibleGroupsCount(15);
    }
  }, [dateGroups.length, debouncedSearchTerm]);

  const loadRemainingSessionPages = useCallback(
    async (initialCursor: string, loadId: number, keyword?: string) => {
      let cursor: string | null = initialCursor;
      setIsPrefetchingSessions(true);

      try {
        while (cursor && loadGenerationRef.current === loadId) {
          const response = await acpListSessions(cursor, {
            keyword,
            archiveState: mode,
            includeLastMessageSnippet: mode === 'archived',
          });
          if (loadGenerationRef.current !== loadId) return;

          cursor = response.nextCursor;
          startTransition(() => {
            setSessions((prev) => {
              const seen = new Set(prev.map((session) => session.id));
              return [
                ...prev,
                ...response.sessions.filter((session) => !seen.has(session.id)),
              ];
            });
          });
        }
      } catch (loadError) {
        console.error('Failed to load remaining sessions:', loadError);
      } finally {
        if (loadGenerationRef.current === loadId) {
          setIsPrefetchingSessions(false);
        }
      }
    },
    [mode]
  );

  const loadSessions = useCallback(
    async (keyword: string = debouncedSearchTermRef.current) => {
      const loadId = loadGenerationRef.current + 1;
      loadGenerationRef.current = loadId;
      const isFirstLoad = !hasLoadedRef.current;
      setIsPrefetchingSessions(false);
      setError(null);
      if (isFirstLoad) {
        setIsLoading(true);
        setShowSkeleton(true);
        setShowContent(false);
      }

      try {
        const response = await acpListSessions(undefined, {
          keyword,
          archiveState: mode,
          includeLastMessageSnippet: mode === 'archived',
        });
        if (loadGenerationRef.current !== loadId) return;

        hasLoadedRef.current = true;
        startTransition(() => {
          setSessions(response.sessions);
        });

        if (response.nextCursor) {
          void loadRemainingSessionPages(response.nextCursor, loadId, keyword);
        }
      } catch (loadError) {
        if (loadGenerationRef.current !== loadId) return;

        console.error('Failed to load sessions:', loadError);
        setError('Failed to load sessions. Please try again later.');
        setSessions([]);
      } finally {
        if (loadGenerationRef.current === loadId && isFirstLoad) {
          setIsLoading(false);
        }
      }
    },
    [loadRemainingSessionPages, mode]
  );

  useEffect(() => {
    void loadSessions(debouncedSearchTerm);
    return () => {
      loadGenerationRef.current += 1;
    };
  }, [debouncedSearchTerm, loadSessions]);

  useEffect(() => {
    const handleSessionUpdate = () => {
      void loadSessions();
    };

    const events =
      mode === 'active'
        ? [
            AppEvents.SESSION_CREATED,
            AppEvents.SESSION_DELETED,
            AppEvents.SESSION_ARCHIVED,
            AppEvents.SESSION_UNARCHIVED,
            AppEvents.SESSION_RENAMED,
          ]
        : [
            AppEvents.SESSION_DELETED,
            AppEvents.SESSION_ARCHIVED,
            AppEvents.SESSION_UNARCHIVED,
            AppEvents.SESSION_RENAMED,
          ];

    events.forEach((eventName) => {
      window.addEventListener(eventName, handleSessionUpdate);
    });

    return () => {
      events.forEach((eventName) => {
        window.removeEventListener(eventName, handleSessionUpdate);
      });
    };
  }, [loadSessions, mode]);

  useEffect(() => {
    if (mode !== 'active') {
      return;
    }

    const config = window.electron.getConfig();
    if (config.GOSLING_DISABLE_NOSTR_SHARING === true) {
      setNostrEnabled(false);
    }
  }, [mode]);

  useEffect(() => {
    if (!isLoading && showSkeleton) {
      setShowSkeleton(false);
      startTransition(() => {
        setTimeout(() => {
          setShowContent(true);
        }, 10);
      });
    }
    return () => void 0;
  }, [isLoading, showSkeleton]);

  const memoizedDateGroups = useMemo(() => {
    if (sessions.length === 0) {
      return [];
    }

    return groupSessionsByDate(
      sessions,
      mode === 'archived' ? archiveTimestamp : sessionActivityAt
    );
  }, [mode, sessions]);

  useEffect(() => {
    startTransition(() => {
      setDateGroups(memoizedDateGroups);
    });
  }, [memoizedDateGroups]);

  const handleScroll = useCallback(
    (target: HTMLDivElement) => {
      const { scrollTop, scrollHeight, clientHeight } = target;
      if (scrollHeight - scrollTop - clientHeight >= 200) {
        return;
      }

      if (visibleGroupsCount < dateGroups.length) {
        setVisibleGroupsCount((prev) => Math.min(prev + 5, dateGroups.length));
      }
    },
    [dateGroups.length, visibleGroupsCount]
  );

  const handleModalSave = useCallback(async (sessionId: string, newDescription: string) => {
    setSessions((prevSessions) =>
      prevSessions.map((session) =>
        session.id === sessionId ? { ...session, name: newDescription, userSetName: true } : session
      )
    );
    window.dispatchEvent(
      new CustomEvent(AppEvents.SESSION_RENAMED, {
        detail: { sessionId, newName: newDescription, userInitiated: true },
      })
    );
  }, []);

  const handleDeleteSession = useCallback(async (session: SessionListItem) => {
    const trackedFilePath = await getTrackedArchiveFile(session.id);
    setDeleteDialogState({
      session,
      filePath: trackedFilePath,
      hasTrackedArchiveFile: trackedFilePath != null,
    });
  }, []);

  const handleConfirmDelete = useCallback(async () => {
    if (!deleteDialogState) {
      return;
    }

    const { session, hasTrackedArchiveFile } = deleteDialogState;
    setDeleteDialogState(null);

    try {
      await acpDeleteSession(session.id);
      const archiveFileRemoval = await removeTrackedArchiveFile(session.id);
      cancelAcpPermissionRequestsForSession(session.id);
      cancelAcpElicitationRequestsForSession(session.id);
      acpChatSessionActions.deleteSnapshot(session.id);
      window.dispatchEvent(
        new CustomEvent(AppEvents.SESSION_DELETED, { detail: { sessionId: session.id } })
      );
      toast.success(intl.formatMessage(i18n.deleteSuccess));
      if (hasTrackedArchiveFile && !archiveFileRemoval.removed) {
        toast.error(intl.formatMessage(i18n.deleteArchiveFileFailed));
      }
    } catch (deleteError) {
      toast.error(
        intl.formatMessage(i18n.deleteFailed, {
          name: session.name,
          error: errorMessage(deleteError, 'Unknown error'),
        })
      );
    }

    await loadSessions();
  }, [deleteDialogState, intl, loadSessions]);

  const handleDuplicateSession = useCallback(
    async (session: SessionListItem) => {
      try {
        await acpForkSession(session.id);
        toast.success(intl.formatMessage(i18n.duplicateSuccess, { name: session.name }));
        window.dispatchEvent(new CustomEvent(AppEvents.SESSION_CREATED));
        await loadSessions();
      } catch (duplicateError) {
        toast.error(
          intl.formatMessage(i18n.duplicateFailed, {
            error: errorMessage(duplicateError, 'Unknown error'),
          })
        );
      }
    },
    [intl, loadSessions]
  );

  const handleArchiveSession = useCallback(
    async (session: SessionListItem) => {
      const folder = await getArchiveFolder();
      if (!folder) {
        toast.error(intl.formatMessage(i18n.archiveFolderMissing));
        navigate('/settings?section=app');
        return;
      }

      setArchiveFolder(folder);
      setArchiveSession(session);
    },
    [intl, navigate]
  );

  const handleConfirmArchive = useCallback(async () => {
    if (!archiveSession) {
      return;
    }

    const session = archiveSession;
    setArchiveSession(null);

    try {
      await archiveSessionToConfiguredFolder(session.id, session.name);
      cancelAcpPermissionRequestsForSession(session.id);
      cancelAcpElicitationRequestsForSession(session.id);
      acpChatSessionActions.deleteSnapshot(session.id);
      window.dispatchEvent(
        new CustomEvent(AppEvents.SESSION_ARCHIVED, { detail: { sessionId: session.id } })
      );
      toast.success(intl.formatMessage(i18n.archiveSuccess));
    } catch (archiveError) {
      if (archiveError instanceof ArchiveFolderNotConfiguredError) {
        toast.error(intl.formatMessage(i18n.archiveFolderMissing));
        navigate('/settings?section=app');
      } else {
        toast.error(
          intl.formatMessage(i18n.archiveFailed, {
            name: session.name,
            error: errorMessage(archiveError, 'Unknown error'),
          })
        );
      }
    }

    await loadSessions();
  }, [archiveSession, intl, loadSessions, navigate]);

  const handleRestoreSession = useCallback((session: SessionListItem) => {
    setRestoreSession(session);
  }, []);

  const handleConfirmRestore = useCallback(async () => {
    if (!restoreSession) {
      return;
    }

    const session = restoreSession;
    setRestoreSession(null);

    try {
      await acpUnarchiveSession(session.id);
      window.dispatchEvent(
        new CustomEvent(AppEvents.SESSION_UNARCHIVED, {
          detail: {
            sessionId: session.id,
            session: {
              ...session,
              archivedAt: undefined,
            },
          },
        })
      );
      toast.success(intl.formatMessage(i18n.restoreSuccess));
    } catch (restoreError) {
      toast.error(
        intl.formatMessage(i18n.restoreFailed, {
          name: session.name,
          error: errorMessage(restoreError, 'Unknown error'),
        })
      );
    }

    await loadSessions();
  }, [intl, loadSessions, restoreSession]);

  const handleExportSession = useCallback(
    async (session: SessionListItem, event: React.MouseEvent) => {
      event.stopPropagation();

      const json = await acpExportSession(session.id);
      const blob = new Blob([json], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement('a');
      anchor.href = url;
      anchor.download = `${session.name}.json`;
      document.body.appendChild(anchor);
      anchor.click();
      document.body.removeChild(anchor);
      URL.revokeObjectURL(url);
      toast.success(intl.formatMessage(i18n.exportSuccess));
    },
    [intl]
  );

  const handleShareSessionNostr = useCallback(
    async (session: SessionListItem, event: React.MouseEvent) => {
      event.stopPropagation();
      setSharingSessionId(session.id);
      try {
        const response = await acpShareSessionNostr(session.id, []);
        setShareLink(response.deeplink);
        setShowShareLinkModal(true);
        toast.success(intl.formatMessage(i18n.shareNostrSuccess));
      } catch (shareError) {
        toast.error(
          intl.formatMessage(i18n.shareNostrFailed, {
            error: errorMessage(shareError, 'Unknown error'),
          })
        );
      } finally {
        setSharingSessionId(null);
      }
    },
    [intl]
  );

  const handleOpenInNewWindow = useCallback((session: SessionListItem, event: React.MouseEvent) => {
    event.stopPropagation();
    window.electron.createChatWindow({
      dir: session.workingDir,
      resumeSessionId: session.id,
      viewType: 'pair',
    });
  }, []);

  const SessionItem = React.memo(function SessionItem({ session }: { session: SessionListItem }) {
    const displayName = session.name;
    const snippet = session.lastMessageSnippet?.trim()
      ? session.lastMessageSnippet
      : intl.formatMessage(i18n.archivedSnippetFallback);
    const handleCardClick = () => {
      if (mode === 'active' && onSelectSession) {
        onSelectSession(session.id);
      }
    };

    return (
      <Card
        onClick={handleCardClick}
        className={`h-full py-3 px-4 transition-all duration-150 flex flex-col justify-between relative group ${
          mode === 'active' ? 'cursor-pointer hover:shadow-default' : ''
        }`}
      >
        <div>
          <h3 className="mb-1 w-full break-words text-base line-clamp-2">{displayName}</h3>
          <div className="mt-2 flex-1">
            <div className="flex items-center text-xs text-text-secondary">
              <Calendar className="mr-1 h-3 w-3 flex-shrink-0" />
              <span>
                {formatMessageTimestamp(
                  Date.parse(mode === 'archived' ? archiveTimestamp(session) : sessionActivityAt(session)) /
                    1000
                )}
              </span>
            </div>
            <div className="flex items-center text-xs text-text-secondary">
              <Folder className="mr-1 h-3 w-3 flex-shrink-0" />
              <span className="truncate">{session.workingDir}</span>
            </div>
            {mode === 'archived' && (
              <>
                <div className="mt-2 text-xs font-medium text-text-secondary">
                  {intl.formatMessage(i18n.archivedAtLabel)}
                </div>
                <p className="mt-1 text-sm text-text-primary line-clamp-3">{snippet}</p>
              </>
            )}
          </div>
        </div>

        <div className="mt-1 flex items-center justify-between">
          <div className="flex items-center space-x-3 text-xs text-text-secondary">
            <div className="flex items-center">
              <MessageSquareText className="mr-1 h-3 w-3" />
              <span className="font-mono">{session.messageCount}</span>
            </div>
          </div>
        </div>

        {mode === 'active' ? (
          <div className="flex justify-end gap-1 opacity-0 transition-opacity group-hover:opacity-100">
            <button
              onClick={(event) => handleOpenInNewWindow(session, event)}
              className="cursor-pointer rounded p-2 hover:bg-gray-100 dark:hover:bg-gray-700"
              title={intl.formatMessage(i18n.openInNewWindow)}
            >
              <ExternalLink className="h-3 w-3 text-text-secondary hover:text-text-primary" />
            </button>
            <button
              onClick={(event) => {
                event.stopPropagation();
                setEditingSession(session);
                setShowEditModal(true);
              }}
              className="cursor-pointer rounded p-2 hover:bg-gray-100 dark:hover:bg-gray-700"
              title={intl.formatMessage(i18n.editSessionName)}
            >
              <Edit2 className="h-3 w-3 text-text-secondary hover:text-text-primary" />
            </button>
            <button
              onClick={(event) => {
                event.stopPropagation();
                void handleDuplicateSession(session);
              }}
              className="cursor-pointer rounded p-2 hover:bg-gray-100 dark:hover:bg-gray-700"
              title={intl.formatMessage(i18n.duplicateSession)}
            >
              <Copy className="h-3 w-3 text-text-secondary hover:text-text-primary" />
            </button>
            <button
              onClick={(event) => {
                event.stopPropagation();
                void handleArchiveSession(session);
              }}
              className="cursor-pointer rounded p-2 hover:bg-amber-50 dark:hover:bg-amber-900/20"
              title={intl.formatMessage(i18n.archiveSession)}
            >
              <Archive className="h-3 w-3 text-amber-600 hover:text-amber-700" />
            </button>
            <button
              onClick={(event) => {
                event.stopPropagation();
                void handleDeleteSession(session);
              }}
              className="cursor-pointer rounded p-2 transition-colors hover:bg-red-50 dark:hover:bg-red-900/20"
              title={intl.formatMessage(i18n.deleteSession)}
            >
              <Trash2 className="h-3 w-3 text-red-500 hover:text-red-600" />
            </button>
            <button
              onClick={(event) => void handleExportSession(session, event)}
              className="cursor-pointer rounded p-2 hover:bg-gray-100 dark:hover:bg-gray-700"
              title={intl.formatMessage(i18n.exportSession)}
            >
              <Download className="h-3 w-3 text-text-secondary hover:text-text-primary" />
            </button>
            {nostrEnabled && (
              <button
                onClick={(event) => void handleShareSessionNostr(session, event)}
                disabled={sharingSessionId === session.id}
                className="cursor-pointer rounded p-2 hover:bg-gray-100 disabled:cursor-wait disabled:opacity-60 dark:hover:bg-gray-700"
                title={intl.formatMessage(i18n.shareNostrSession)}
              >
                {sharingSessionId === session.id ? (
                  <LoaderCircle className="h-3 w-3 animate-spin text-text-secondary" />
                ) : (
                  <Share2 className="h-3 w-3 text-text-secondary hover:text-text-primary" />
                )}
              </button>
            )}
          </div>
        ) : (
          <div className="flex justify-end gap-1 opacity-0 transition-opacity group-hover:opacity-100">
            <button
              onClick={(event) => {
                event.stopPropagation();
                handleRestoreSession(session);
              }}
              className="cursor-pointer rounded p-2 hover:bg-emerald-50 dark:hover:bg-emerald-900/20"
              title={intl.formatMessage(i18n.restoreSession)}
            >
              <RotateCcw className="h-3 w-3 text-emerald-600 hover:text-emerald-700" />
            </button>
            <button
              onClick={(event) => {
                event.stopPropagation();
                void handleDeleteSession(session);
              }}
              className="cursor-pointer rounded p-2 transition-colors hover:bg-red-50 dark:hover:bg-red-900/20"
              title={intl.formatMessage(i18n.deleteSession)}
            >
              <Trash2 className="h-3 w-3 text-red-500 hover:text-red-600" />
            </button>
            <button
              onClick={(event) => void handleExportSession(session, event)}
              className="cursor-pointer rounded p-2 hover:bg-gray-100 dark:hover:bg-gray-700"
              title={intl.formatMessage(i18n.exportSession)}
            >
              <Download className="h-3 w-3 text-text-secondary hover:text-text-primary" />
            </button>
          </div>
        )}
      </Card>
    );
  });

  const SessionSkeleton = React.memo(({ variant = 0 }: { variant?: number }) => {
    const titleWidths = ['w-3/4', 'w-2/3', 'w-4/5', 'w-1/2'];
    const pathWidths = ['w-32', 'w-28', 'w-36', 'w-24'];
    const tokenWidths = ['w-12', 'w-10', 'w-14', 'w-8'];

    return (
      <Card className="session-skeleton flex h-full flex-col justify-between px-4 py-3">
        <div className="flex-1">
          <Skeleton className={`mb-2 h-5 ${titleWidths[variant % titleWidths.length]}`} />
          <div className="mb-1 flex items-center">
            <Skeleton className="mr-1 h-3 w-3 rounded-sm" />
            <Skeleton className="h-4 w-20" />
          </div>
          <div className="mb-1 flex items-center">
            <Skeleton className="mr-1 h-3 w-3 rounded-sm" />
            <Skeleton className={`h-4 ${pathWidths[variant % pathWidths.length]}`} />
          </div>
        </div>

        <div className="mt-1 flex items-center justify-between pt-2">
          <div className="flex items-center space-x-3">
            <div className="flex items-center">
              <Skeleton className="mr-1 h-3 w-3 rounded-sm" />
              <Skeleton className="h-4 w-8" />
            </div>
            <div className="flex items-center">
              <Skeleton className="mr-1 h-3 w-3 rounded-sm" />
              <Skeleton className={`h-4 ${tokenWidths[variant % tokenWidths.length]}`} />
            </div>
          </div>
        </div>
      </Card>
    );
  });

  SessionSkeleton.displayName = 'SessionSkeleton';

  const renderActualContent = () => {
    if (error) {
      return (
        <div className="flex h-full flex-col items-center justify-center text-text-secondary">
          <MessageSquareText className="mb-4 h-12 w-12 text-red-500" />
          <p className="mb-2 text-lg">{intl.formatMessage(i18n.errorLoading)}</p>
          <p className="mb-4 text-center text-sm">{error}</p>
          <Button onClick={() => void loadSessions(debouncedSearchTerm)} variant="default">
            {intl.formatMessage(i18n.tryAgain)}
          </Button>
        </div>
      );
    }

    if (sessions.length === 0) {
      if (debouncedSearchTerm) {
        return (
          <div className="mt-4 flex h-full flex-col items-center justify-center text-text-secondary">
            <MessageSquareText className="mb-4 h-12 w-12" />
            <p className="mb-2 text-lg">
              {intl.formatMessage(
                mode === 'archived' ? i18n.noMatchingArchives : i18n.noMatching
              )}
            </p>
            <p className="text-sm">{intl.formatMessage(i18n.noMatchingDesc)}</p>
          </div>
        );
      }

      return (
        <div className="flex h-full flex-col justify-center text-text-secondary">
          <MessageSquareText className="mb-4 h-12 w-12" />
          <p className="mb-2 text-lg">
            {intl.formatMessage(mode === 'archived' ? i18n.noArchivedSessions : i18n.noSessions)}
          </p>
          <p className="text-sm">
            {intl.formatMessage(
              mode === 'archived' ? i18n.noArchivedSessionsDesc : i18n.noSessionsDesc
            )}
          </p>
        </div>
      );
    }

    return (
      <div className="space-y-8">
        {visibleDateGroups.map((group) => (
          <div key={group.label} className="space-y-4">
            <div className="sticky top-0 z-10 bg-background-primary/95 backdrop-blur-sm">
              <h2 className="text-text-secondary">{group.label}</h2>
            </div>
            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5">
              {group.sessions.map((session) => (
                <SessionItem key={session.id} session={session} />
              ))}
            </div>
          </div>
        ))}

        {isPrefetchingSessions && (
          <div className="flex justify-center py-8">
            <div className="flex items-center space-x-2 text-text-secondary">
              <div className="h-4 w-4 animate-spin rounded-full border-b-2" />
              <span>{intl.formatMessage(i18n.loadingMore)}</span>
            </div>
          </div>
        )}
      </div>
    );
  };

  return (
    <>
      <div className="flex-1 min-h-0 relative">
        <ScrollArea handleScroll={handleScroll} className="h-full" data-search-scroll-area>
          <div className="relative h-full px-8">
            <SearchView
              onSearch={setSearchTerm}
              className="relative"
              placeholder={intl.formatMessage(
                mode === 'archived' ? i18n.searchArchivesPlaceholder : i18n.searchPlaceholder
              )}
              showCaseSensitive={false}
              showNavigation={false}
              highlightMatches={false}
            >
              <div
                className={`absolute inset-0 transition-opacity duration-300 ${
                  isLoading || showSkeleton ? 'opacity-100 z-10' : 'opacity-0 z-0 pointer-events-none'
                }`}
              >
                <div className="space-y-8">
                  <div className="space-y-4">
                    <Skeleton className="h-6 w-16" />
                    <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5">
                      <SessionSkeleton variant={0} />
                      <SessionSkeleton variant={1} />
                      <SessionSkeleton variant={2} />
                      <SessionSkeleton variant={3} />
                      <SessionSkeleton variant={0} />
                    </div>
                  </div>
                  <div className="space-y-4">
                    <Skeleton className="h-6 w-20" />
                    <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5">
                      <SessionSkeleton variant={1} />
                      <SessionSkeleton variant={2} />
                      <SessionSkeleton variant={3} />
                      <SessionSkeleton variant={0} />
                    </div>
                  </div>
                </div>
              </div>

              <div
                className={`relative transition-opacity duration-300 ${
                  showContent ? 'opacity-100 z-10' : 'opacity-0 z-0'
                }`}
              >
                {renderActualContent()}
              </div>
            </SearchView>
          </div>
        </ScrollArea>
      </div>

      <EditSessionModal
        session={editingSession}
        isOpen={showEditModal}
        onClose={() => {
          setShowEditModal(false);
          setEditingSession(null);
        }}
        onSave={handleModalSave}
      />

      <Dialog open={showShareLinkModal} onOpenChange={setShowShareLinkModal}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Share2 className="h-5 w-5" />
              {intl.formatMessage(i18n.shareNostrTitle)}
            </DialogTitle>
            <DialogDescription>{intl.formatMessage(i18n.shareNostrDesc)}</DialogDescription>
          </DialogHeader>

          <div className="relative rounded-lg border border-border-primary bg-background-secondary p-3 pr-12">
            <code className="block max-h-36 overflow-y-auto break-all text-sm text-text-primary">
              {shareLink}
            </code>
            <Button
              variant="ghost"
              size="sm"
              className="absolute right-2 top-2"
              onClick={async () => {
                try {
                  await navigator.clipboard.writeText(shareLink);
                  toast.success(intl.formatMessage(i18n.copied));
                } catch (copyError) {
                  toast.error(`Failed to copy: ${errorMessage(copyError, 'Unknown error')}`);
                }
              }}
              disabled={!shareLink}
            >
              <Copy className="h-4 w-4" />
              <span className="sr-only">{intl.formatMessage(i18n.copied)}</span>
            </Button>
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => setShowShareLinkModal(false)}>
              {intl.formatMessage(i18n.close)}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <ConfirmationModal
        isOpen={deleteDialogState != null}
        title={intl.formatMessage(i18n.deleteTitle)}
        message={
          deleteDialogState?.hasTrackedArchiveFile
            ? intl.formatMessage(i18n.deleteWithArchiveFileMessage, {
                name: deleteDialogState.session.name,
              })
            : intl.formatMessage(i18n.deleteMessage, {
                name: deleteDialogState?.session.name ?? '',
              })
        }
        detail={
          deleteDialogState?.hasTrackedArchiveFile && deleteDialogState.filePath ? (
            <div>
              {intl.formatMessage(i18n.deleteTrackedFileDetail, {
                filePath: deleteDialogState.filePath,
              })}
            </div>
          ) : undefined
        }
        confirmLabel={intl.formatMessage(i18n.deleteTitle)}
        cancelLabel={intl.formatMessage(i18n.cancel)}
        confirmVariant="destructive"
        onConfirm={() => void handleConfirmDelete()}
        onCancel={() => setDeleteDialogState(null)}
      />

      <ConfirmationModal
        isOpen={archiveSession != null}
        title={intl.formatMessage(i18n.archiveTitle)}
        message={intl.formatMessage(i18n.archiveMessage, { name: archiveSession?.name ?? '' })}
        detail={
          archiveFolder ? (
            <div>{intl.formatMessage(i18n.archiveDetail, { folder: archiveFolder })}</div>
          ) : undefined
        }
        confirmLabel={intl.formatMessage(i18n.archiveTitle)}
        cancelLabel={intl.formatMessage(i18n.cancel)}
        onConfirm={() => void handleConfirmArchive()}
        onCancel={() => setArchiveSession(null)}
      />

      <ConfirmationModal
        isOpen={restoreSession != null}
        title={intl.formatMessage(i18n.restoreTitle)}
        message={intl.formatMessage(i18n.restoreMessage, { name: restoreSession?.name ?? '' })}
        confirmLabel={intl.formatMessage(i18n.restoreTitle)}
        cancelLabel={intl.formatMessage(i18n.cancel)}
        onConfirm={() => void handleConfirmRestore()}
        onCancel={() => setRestoreSession(null)}
      />
    </>
  );
}
