export type PersistedTerminalTab = {
  id: string;
  title: string;
};

export type PersistedTerminalState = {
  open: boolean;
  width: number;
  tabs: PersistedTerminalTab[];
  activeTabId: string | null;
};

export const TERMINAL_DEFAULT_WIDTH = 420;
export const TERMINAL_MIN_WIDTH = 280;

function storageKey(sessionId: string): string {
  return `goose.terminal.panel.${sessionId}`;
}

export function loadTerminalState(sessionId: string): PersistedTerminalState {
  try {
    const raw = localStorage.getItem(storageKey(sessionId));
    if (!raw) {
      return {
        open: false,
        width: TERMINAL_DEFAULT_WIDTH,
        tabs: [],
        activeTabId: null,
      };
    }
    const parsed = JSON.parse(raw) as Partial<PersistedTerminalState>;
    const width =
      typeof parsed.width === 'number' && Number.isFinite(parsed.width)
        ? Math.max(TERMINAL_MIN_WIDTH, parsed.width)
        : TERMINAL_DEFAULT_WIDTH;
    const tabs = Array.isArray(parsed.tabs)
      ? parsed.tabs.filter(
          (tab): tab is PersistedTerminalTab =>
            !!tab && typeof tab.id === 'string' && typeof tab.title === 'string'
        )
      : [];
    return {
      open: Boolean(parsed.open),
      width,
      tabs,
      activeTabId:
        typeof parsed.activeTabId === 'string'
          ? parsed.activeTabId
          : (tabs[0]?.id ?? null),
    };
  } catch {
    return {
      open: false,
      width: TERMINAL_DEFAULT_WIDTH,
      tabs: [],
      activeTabId: null,
    };
  }
}

export function saveTerminalState(sessionId: string, state: PersistedTerminalState): void {
  try {
    localStorage.setItem(storageKey(sessionId), JSON.stringify(state));
  } catch {
    // quota / private mode — ignore
  }
}

export function clearTerminalState(sessionId: string): void {
  try {
    localStorage.removeItem(storageKey(sessionId));
  } catch {
    // ignore
  }
}

export function newTerminalTabId(): string {
  return `term_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`;
}
