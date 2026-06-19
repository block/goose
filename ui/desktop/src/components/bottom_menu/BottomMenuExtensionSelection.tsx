import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useConfig, type FixedExtensionEntry } from '../ConfigContext';
import { toastService } from '../../toasts';
import { formatExtensionName } from '../settings/extensions/subcomponents/ExtensionList';
import { nameToKey } from '../settings/extensions/utils';
import { ExtensionConfig, getSessionExtensions } from '../../api';
import { addToAgent, removeFromAgent } from '../settings/extensions/agent-api';
import { defineMessages, useIntl } from '../../i18n';
import { AppEvents } from '../../constants/events';
import { ExtensionMenu } from './ExtensionMenu';
import {
  isNextChatExtensionSelected,
  toggleNextChatExtension,
  type NextChatExtensionDraft,
} from '../../utils/nextChatExtensions';

const i18n = defineMessages({
  manageExtensions: {
    id: 'bottomMenuExtensionSelection.manageExtensions',
    defaultMessage: 'manage extensions',
  },
  searchExtensions: {
    id: 'bottomMenuExtensionSelection.searchExtensions',
    defaultMessage: 'search extensions...',
  },
  extensionsForNewChats: {
    id: 'bottomMenuExtensionSelection.extensionsForNewChats',
    defaultMessage: 'Extensions for new chats',
  },
  extensionsForThisSession: {
    id: 'bottomMenuExtensionSelection.extensionsForThisSession',
    defaultMessage: 'Extensions for this chat session',
  },
  noExtensionsFound: {
    id: 'bottomMenuExtensionSelection.noExtensionsFound',
    defaultMessage: 'no extensions found',
  },
  noExtensionsAvailable: {
    id: 'bottomMenuExtensionSelection.noExtensionsAvailable',
    defaultMessage: 'no extensions available',
  },
  extensionUpdated: {
    id: 'bottomMenuExtensionSelection.extensionUpdated',
    defaultMessage: 'Extension Updated',
  },
  extensionWillBeEnabled: {
    id: 'bottomMenuExtensionSelection.extensionWillBeEnabled',
    defaultMessage: '{name} will be enabled in new chats',
  },
  extensionWillBeDisabled: {
    id: 'bottomMenuExtensionSelection.extensionWillBeDisabled',
    defaultMessage: '{name} will be disabled in new chats',
  },
});

interface BottomMenuExtensionSelectionProps {
  sessionId: string | null;
  nextChatExtensionDraft?: NextChatExtensionDraft;
  onNextChatExtensionDraftChange?: (draft: NextChatExtensionDraft) => void;
}

type GetSessionExtensionsSignal = Parameters<typeof getSessionExtensions>[0]['signal'];

export const BottomMenuExtensionSelection = ({
  sessionId,
  nextChatExtensionDraft,
  onNextChatExtensionDraftChange,
}: BottomMenuExtensionSelectionProps) => {
  if (!sessionId) {
    if (!nextChatExtensionDraft) {
      return null;
    }

    return (
      <DraftExtensionsMenu
        draft={nextChatExtensionDraft}
        onDraftChange={onNextChatExtensionDraftChange ?? (() => {})}
      />
    );
  }

  return <SessionExtensionsMenu sessionId={sessionId} />;
};

function DraftExtensionsMenu({
  draft,
  onDraftChange,
}: {
  draft: NextChatExtensionDraft;
  onDraftChange: (draft: NextChatExtensionDraft) => void;
}) {
  const intl = useIntl();
  const { extensionsList: allExtensions } = useConfig();
  const [visibleDraft, setVisibleDraft] = useState<NextChatExtensionDraft>(draft);
  const [isTransitioning, setIsTransitioning] = useState(false);
  const [isSortPending, setIsSortPending] = useState(false);
  const [togglingExtensionName, setTogglingExtensionName] = useState<string | null>(null);
  const sortTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const clearSortTimeout = useCallback(() => {
    if (sortTimeoutRef.current) {
      clearTimeout(sortTimeoutRef.current);
      sortTimeoutRef.current = null;
    }
  }, []);

  const resetTransition = useCallback(() => {
    clearSortTimeout();
    setIsTransitioning(false);
    setIsSortPending(false);
    setTogglingExtensionName(null);
  }, [clearSortTimeout]);

  useEffect(() => clearSortTimeout, [clearSortTimeout]);

  useEffect(() => {
    if (!isTransitioning) {
      setVisibleDraft(draft);
    }
  }, [draft, isTransitioning]);

  const handleToggle = useCallback(
    (extensionConfig: FixedExtensionEntry) => {
      if (togglingExtensionName === extensionConfig.name) {
        return;
      }

      setIsTransitioning(true);
      setTogglingExtensionName(extensionConfig.name);

      const currentState = isNextChatExtensionSelected(extensionConfig, draft);
      const nextDraft = toggleNextChatExtension(draft, extensionConfig);
      onDraftChange(nextDraft);
      setIsSortPending(true);

      if (sortTimeoutRef.current) {
        clearTimeout(sortTimeoutRef.current);
      }

      sortTimeoutRef.current = setTimeout(() => {
        setVisibleDraft(nextDraft);
        setIsSortPending(false);
        setIsTransitioning(false);
        setTogglingExtensionName(null);
        sortTimeoutRef.current = null;
      }, 800);

      toastService.success({
        title: intl.formatMessage(i18n.extensionUpdated),
        msg: intl.formatMessage(
          !currentState ? i18n.extensionWillBeEnabled : i18n.extensionWillBeDisabled,
          { name: formatExtensionName(extensionConfig.name) }
        ),
      });
    },
    [draft, intl, onDraftChange, togglingExtensionName]
  );

  const extensions = useMemo(() => {
    return allExtensions.map(
      (extension) =>
        ({
          ...extension,
          enabled: isNextChatExtensionSelected(extension, visibleDraft),
        }) as FixedExtensionEntry
    );
  }, [allExtensions, visibleDraft]);

  return (
    <ExtensionMenu
      extensions={extensions}
      title={intl.formatMessage(i18n.manageExtensions)}
      searchPlaceholder={intl.formatMessage(i18n.searchExtensions)}
      description={intl.formatMessage(i18n.extensionsForNewChats)}
      emptyMessage={intl.formatMessage(i18n.noExtensionsAvailable)}
      noResultsMessage={intl.formatMessage(i18n.noExtensionsFound)}
      hidden={extensions.length === 0}
      isTransitioning={isTransitioning}
      isSortPending={isSortPending}
      togglingExtensionName={togglingExtensionName}
      onToggle={handleToggle}
      onClose={resetTransition}
    />
  );
}

function SessionExtensionsMenu({ sessionId }: { sessionId: string }) {
  const intl = useIntl();
  const [sessionExtensions, setSessionExtensions] = useState<ExtensionConfig[]>([]);
  const [isTransitioning, setIsTransitioning] = useState(false);
  const [isSortPending, setIsSortPending] = useState(false);
  const [togglingExtensionName, setTogglingExtensionName] = useState<string | null>(null);
  const [isSessionExtensionsLoaded, setIsSessionExtensionsLoaded] = useState(false);
  const sortTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const latestSessionIdRef = useRef(sessionId);
  const { extensionsList: allExtensions } = useConfig();

  const clearSortTimeout = useCallback(() => {
    if (sortTimeoutRef.current) {
      clearTimeout(sortTimeoutRef.current);
      sortTimeoutRef.current = null;
    }
  }, []);

  const resetTransition = useCallback(() => {
    clearSortTimeout();
    setIsTransitioning(false);
    setIsSortPending(false);
    setTogglingExtensionName(null);
  }, [clearSortTimeout]);

  useEffect(() => {
    latestSessionIdRef.current = sessionId;
    setIsSessionExtensionsLoaded(false);
    setSessionExtensions([]);
    resetTransition();
  }, [sessionId, resetTransition]);

  useEffect(() => clearSortTimeout, [clearSortTimeout]);

  const loadSessionExtensions = useCallback(
    async (targetSessionId: string, signal?: GetSessionExtensionsSignal) => {
      const response = await getSessionExtensions({
        path: { session_id: targetSessionId },
        signal,
        throwOnError: true,
      });

      if (signal?.aborted || latestSessionIdRef.current !== targetSessionId) {
        return;
      }

      setSessionExtensions(response.data?.extensions ?? []);
      setIsSessionExtensionsLoaded(true);
    },
    []
  );

  useEffect(() => {
    let controller: AbortController | null = null;

    const loadForSession = (targetSessionId: string) => {
      controller?.abort();
      const currentController = new AbortController();
      controller = currentController;

      loadSessionExtensions(targetSessionId, currentController.signal).catch((error) => {
        if (currentController.signal.aborted || latestSessionIdRef.current !== targetSessionId) {
          return;
        }

        console.error('Failed to fetch session extensions:', error);
        setIsSessionExtensionsLoaded(true);
      });
    };

    const loadExtensionsForCurrentSession = (event: Event) => {
      const targetSessionId = (event as CustomEvent<{ sessionId?: string }>).detail?.sessionId;

      if (targetSessionId !== sessionId) {
        return;
      }

      loadForSession(targetSessionId);
    };

    window.addEventListener(AppEvents.SESSION_EXTENSIONS_LOADED, loadExtensionsForCurrentSession);
    loadForSession(sessionId);

    return () => {
      controller?.abort();
      window.removeEventListener(
        AppEvents.SESSION_EXTENSIONS_LOADED,
        loadExtensionsForCurrentSession
      );
    };
  }, [sessionId, loadSessionExtensions]);

  const finishSessionTransition = useCallback((targetSessionId: string) => {
    if (latestSessionIdRef.current === targetSessionId) {
      setIsSortPending(false);
      setIsTransitioning(false);
      setTogglingExtensionName(null);
    }
  }, []);

  const handleToggle = useCallback(
    async (extensionConfig: FixedExtensionEntry) => {
      if (togglingExtensionName === extensionConfig.name) {
        return;
      }

      setIsTransitioning(true);
      setTogglingExtensionName(extensionConfig.name);

      try {
        if (extensionConfig.enabled) {
          await removeFromAgent(extensionConfig.name, sessionId, true);
        } else {
          await addToAgent(extensionConfig, sessionId, true);
        }

        setIsSortPending(true);

        if (sortTimeoutRef.current) {
          clearTimeout(sortTimeoutRef.current);
        }

        sortTimeoutRef.current = setTimeout(() => {
          loadSessionExtensions(sessionId)
            .catch((error) => {
              if (latestSessionIdRef.current === sessionId) {
                console.error('Failed to fetch session extensions:', error);
              }
            })
            .finally(() => {
              finishSessionTransition(sessionId);
              sortTimeoutRef.current = null;
            });
        }, 800);
      } catch {
        setIsTransitioning(false);
        setIsSortPending(false);
        setTogglingExtensionName(null);
      }
    },
    [finishSessionTransition, loadSessionExtensions, sessionId, togglingExtensionName]
  );

  const extensions = useMemo(() => {
    const sessionExtensionKeys = new Set(
      sessionExtensions.map((extension) => nameToKey(extension.name))
    );
    const configuredExtensionKeys = new Set(
      allExtensions.map((extension) => nameToKey(extension.name))
    );

    const mergedExtensions = allExtensions.map(
      (extension) =>
        ({
          ...extension,
          enabled: sessionExtensionKeys.has(nameToKey(extension.name)),
        }) as FixedExtensionEntry
    );

    for (const sessionExtension of sessionExtensions) {
      if (configuredExtensionKeys.has(nameToKey(sessionExtension.name))) {
        continue;
      }

      mergedExtensions.push({
        ...sessionExtension,
        enabled: true,
      });
    }

    return mergedExtensions;
  }, [allExtensions, sessionExtensions]);

  return (
    <ExtensionMenu
      extensions={extensions}
      title={intl.formatMessage(i18n.manageExtensions)}
      searchPlaceholder={intl.formatMessage(i18n.searchExtensions)}
      description={intl.formatMessage(i18n.extensionsForThisSession)}
      emptyMessage={intl.formatMessage(i18n.noExtensionsAvailable)}
      noResultsMessage={intl.formatMessage(i18n.noExtensionsFound)}
      hidden={extensions.length === 0 || !isSessionExtensionsLoaded}
      isTransitioning={isTransitioning}
      isSortPending={isSortPending}
      togglingExtensionName={togglingExtensionName}
      onToggle={handleToggle}
      onClose={resetTransition}
    />
  );
}
