import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { defineMessages, useIntl } from '../../i18n';
import { cn } from '../../utils';
import { AppEvents } from '../../constants/events';
import { TerminalTabView } from './TerminalTabView';
import {
  TERMINAL_DEFAULT_WIDTH,
  TERMINAL_MIN_WIDTH,
  clearTerminalState,
  loadTerminalState,
  newTerminalTabId,
  saveTerminalState,
  type PersistedTerminalTab,
} from './terminalPersistence';

const i18n = defineMessages({
  terminal: {
    id: 'terminalPanel.terminal',
    defaultMessage: 'Terminal',
  },
  newTab: {
    id: 'terminalPanel.newTab',
    defaultMessage: 'New terminal tab',
  },
  closeTab: {
    id: 'terminalPanel.closeTab',
    defaultMessage: 'Close terminal tab',
  },
  resize: {
    id: 'terminalPanel.resize',
    defaultMessage: 'Resize terminal panel',
  },
});

type TerminalPanelProps = {
  sessionId: string;
  cwd: string;
  isActiveSession: boolean;
  onRequestChatFocus: () => void;
};

export function TerminalPanel({
  sessionId,
  cwd,
  isActiveSession,
  onRequestChatFocus,
}: TerminalPanelProps) {
  const intl = useIntl();
  const initial = useMemo(() => loadTerminalState(sessionId), [sessionId]);
  const [open, setOpen] = useState(initial.open);
  const [width, setWidth] = useState(initial.width || TERMINAL_DEFAULT_WIDTH);
  const [tabs, setTabs] = useState<PersistedTerminalTab[]>(initial.tabs);
  const [activeTabId, setActiveTabId] = useState<string | null>(initial.activeTabId);
  const [focusToken, setFocusToken] = useState(0);
  const draggingRef = useRef(false);
  const openRef = useRef(open);
  openRef.current = open;

  const persist = useCallback(
    (next: {
      open?: boolean;
      width?: number;
      tabs?: PersistedTerminalTab[];
      activeTabId?: string | null;
    }) => {
      const state = {
        open: next.open ?? openRef.current,
        width: next.width ?? width,
        tabs: next.tabs ?? tabs,
        activeTabId: next.activeTabId !== undefined ? next.activeTabId : activeTabId,
      };
      saveTerminalState(sessionId, state);
    },
    [sessionId, width, tabs, activeTabId]
  );

  const ensureTab = useCallback((): string => {
    if (activeTabId && tabs.some((t) => t.id === activeTabId)) {
      return activeTabId;
    }
    if (tabs[0]) {
      setActiveTabId(tabs[0].id);
      return tabs[0].id;
    }
    const id = newTerminalTabId();
    const nextTabs = [{ id, title: '1' }];
    setTabs(nextTabs);
    setActiveTabId(id);
    persist({ tabs: nextTabs, activeTabId: id });
    return id;
  }, [activeTabId, tabs, persist]);

  const openPanel = useCallback(() => {
    ensureTab();
    setOpen(true);
    persist({ open: true });
    setFocusToken((n) => n + 1);
  }, [ensureTab, persist]);

  const closePanel = useCallback(() => {
    setOpen(false);
    persist({ open: false });
    onRequestChatFocus();
  }, [onRequestChatFocus, persist]);

  useEffect(() => {
    if (!isActiveSession) return;

    const onToggle = () => {
      if (openRef.current) {
        closePanel();
      } else {
        openPanel();
      }
    };

    window.electron.on('toggle-terminal', onToggle);
    return () => {
      window.electron.off('toggle-terminal', onToggle);
    };
  }, [isActiveSession, openPanel, closePanel]);

  useEffect(() => {
    const onDeleted = (event: Event) => {
      const detail = (event as CustomEvent<{ sessionId?: string }>).detail;
      if (detail?.sessionId !== sessionId) return;
      void window.electron.terminalKillSession({ sessionId });
      clearTerminalState(sessionId);
    };
    window.addEventListener(AppEvents.SESSION_DELETED, onDeleted);
    return () => window.removeEventListener(AppEvents.SESSION_DELETED, onDeleted);
  }, [sessionId]);

  // Restore open panel after reload: ensure a tab exists so PTY can spawn.
  useEffect(() => {
    if (open && tabs.length === 0) {
      ensureTab();
    }
  }, [open, tabs.length, ensureTab]);

  const addTab = () => {
    const id = newTerminalTabId();
    const nextTabs = [...tabs, { id, title: String(tabs.length + 1) }];
    setTabs(nextTabs);
    setActiveTabId(id);
    setOpen(true);
    persist({ open: true, tabs: nextTabs, activeTabId: id });
    setFocusToken((n) => n + 1);
  };

  const closeTab = (tabId: string) => {
    void window.electron.terminalKill({ sessionId, tabId });
    const nextTabs = tabs.filter((t) => t.id !== tabId).map((t, i) => ({
      ...t,
      title: String(i + 1),
    }));
    if (nextTabs.length === 0) {
      setTabs([]);
      setActiveTabId(null);
      setOpen(false);
      persist({ open: false, tabs: [], activeTabId: null });
      onRequestChatFocus();
      return;
    }
    const nextActive =
      activeTabId === tabId
        ? nextTabs[Math.max(0, tabs.findIndex((t) => t.id === tabId) - 1)]?.id ??
          nextTabs[0].id
        : activeTabId;
    setTabs(nextTabs);
    setActiveTabId(nextActive);
    persist({ tabs: nextTabs, activeTabId: nextActive });
  };

  const onResizeStart = (event: React.MouseEvent) => {
    event.preventDefault();
    draggingRef.current = true;
    const startX = event.clientX;
    const startWidth = width;

    const onMove = (moveEvent: MouseEvent) => {
      if (!draggingRef.current) return;
      const delta = startX - moveEvent.clientX;
      const maxWidth = Math.floor(window.innerWidth * 0.5);
      const next = Math.min(maxWidth, Math.max(TERMINAL_MIN_WIDTH, startWidth + delta));
      setWidth(next);
    };

    const onUp = () => {
      draggingRef.current = false;
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
      setWidth((current) => {
        persist({ width: current });
        return current;
      });
    };

    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  };

  // Keep the panel mounted while tabs exist so xterm + background PTYs survive
  // Cmd+J close and chat switches (CSS hide only).
  if (!open && tabs.length === 0) {
    return null;
  }

  return (
    <aside
      className={cn(
        'relative flex h-full min-h-0 shrink-0 flex-col border-l border-border-primary bg-background-primary',
        !open && 'hidden'
      )}
      style={{ width }}
      aria-label={intl.formatMessage(i18n.terminal)}
      aria-hidden={!open}
    >
      <button
        type="button"
        aria-label={intl.formatMessage(i18n.resize)}
        className="absolute inset-y-0 -left-1 z-20 w-2 cursor-col-resize"
        onMouseDown={onResizeStart}
      />

      <div className="flex h-9 shrink-0 items-center gap-1 border-b border-border-primary px-2">
        <div className="flex min-w-0 flex-1 items-center gap-1 overflow-x-auto">
          {tabs.map((tab) => (
            <div
              key={tab.id}
              className={cn(
                'group flex max-w-[9rem] items-center gap-1 rounded-md px-2 py-1 text-xs',
                tab.id === activeTabId
                  ? 'bg-background-secondary text-text-primary'
                  : 'text-text-secondary hover:bg-background-secondary/60'
              )}
            >
              <button
                type="button"
                className="min-w-0 truncate"
                onClick={() => {
                  setActiveTabId(tab.id);
                  persist({ activeTabId: tab.id });
                  setFocusToken((n) => n + 1);
                }}
              >
                {intl.formatMessage(i18n.terminal)} {tab.title}
              </button>
              <button
                type="button"
                className="rounded px-1 opacity-60 hover:opacity-100"
                aria-label={intl.formatMessage(i18n.closeTab)}
                onClick={() => closeTab(tab.id)}
              >
                ×
              </button>
            </div>
          ))}
        </div>
        <button
          type="button"
          className="rounded px-2 py-1 text-sm text-text-secondary hover:bg-background-secondary hover:text-text-primary"
          aria-label={intl.formatMessage(i18n.newTab)}
          onClick={addTab}
        >
          +
        </button>
      </div>

      <div className="relative min-h-0 flex-1">
        {tabs.map((tab) => (
          <TerminalTabView
            key={tab.id}
            sessionId={sessionId}
            tabId={tab.id}
            cwd={cwd}
            isActive={tab.id === activeTabId}
            focusToken={focusToken}
          />
        ))}
      </div>
    </aside>
  );
}
