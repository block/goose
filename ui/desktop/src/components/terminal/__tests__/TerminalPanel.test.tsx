import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { act, render, screen, waitFor } from '@testing-library/react';
import { useEffect } from 'react';
import { IntlTestWrapper } from '../../../i18n/test-utils';
import { TerminalPanel } from '../TerminalPanel';
import { clearTerminalState, loadTerminalState } from '../terminalPersistence';

const mountCounts = new Map<string, number>();
const unmountCounts = new Map<string, number>();

vi.mock('../TerminalTabView', () => ({
  TerminalTabView: ({
    tabId,
    isPanelOpen,
  }: {
    tabId: string;
    isPanelOpen: boolean;
    sessionId: string;
    cwd: string;
    isActive: boolean;
    focusToken: number;
  }) => {
    useEffect(() => {
      mountCounts.set(tabId, (mountCounts.get(tabId) ?? 0) + 1);
      return () => {
        unmountCounts.set(tabId, (unmountCounts.get(tabId) ?? 0) + 1);
      };
    }, [tabId]);

    return (
      <div
        data-testid={`terminal-tab-view-${tabId}`}
        data-tab-id={tabId}
        data-panel-open={isPanelOpen ? 'true' : 'false'}
      />
    );
  },
}));

type Listener = (...args: unknown[]) => void;

describe('TerminalPanel collapse/reopen preserves the same terminal', () => {
  const sessionId = 'session-collapse-bug';
  const listeners = new Map<string, Set<Listener>>();

  function emit(channel: string) {
    for (const listener of listeners.get(channel) ?? []) {
      listener({});
    }
  }

  beforeEach(() => {
    mountCounts.clear();
    unmountCounts.clear();
    listeners.clear();
    clearTerminalState(sessionId);

    window.electron.on = vi.fn((channel: string, callback: Listener) => {
      if (!listeners.has(channel)) listeners.set(channel, new Set());
      listeners.get(channel)!.add(callback);
    });
    window.electron.off = vi.fn((channel: string, callback: Listener) => {
      listeners.get(channel)?.delete(callback);
    });
    window.electron.terminalCreate = vi.fn(async () => ({ ok: true as const }));
    window.electron.terminalKill = vi.fn(async () => true);
    window.electron.terminalKillSession = vi.fn(async () => true);
  });

  afterEach(() => {
    clearTerminalState(sessionId);
  });

  function renderPanel() {
    return render(
      <IntlTestWrapper>
        <TerminalPanel
          sessionId={sessionId}
          cwd="/tmp/goose"
          isActiveSession
          onRequestChatFocus={vi.fn()}
        />
      </IntlTestWrapper>
    );
  }

  async function openViaShortcut() {
    await act(async () => {
      emit('toggle-terminal');
    });
    const view = await waitFor(() => screen.getByTestId(/terminal-tab-view-/));
    return view.getAttribute('data-tab-id')!;
  }

  it('keeps the same tab mounted across Cmd+J close and reopen (no new PTY tab)', async () => {
    renderPanel();

    const tabId = await openViaShortcut();
    expect(mountCounts.get(tabId)).toBe(1);
    expect(screen.getByTestId(`terminal-tab-view-${tabId}`)).toHaveAttribute(
      'data-panel-open',
      'true'
    );

    await act(async () => {
      emit('toggle-terminal');
    });

    // Collapsed: still mounted, panel marked closed — must not unmount/remount.
    expect(screen.getByTestId(`terminal-tab-view-${tabId}`)).toHaveAttribute(
      'data-panel-open',
      'false'
    );
    expect(mountCounts.get(tabId)).toBe(1);
    expect(unmountCounts.get(tabId) ?? 0).toBe(0);
    expect(window.electron.terminalKill).not.toHaveBeenCalled();

    await act(async () => {
      emit('toggle-terminal');
    });

    expect(screen.getByTestId(`terminal-tab-view-${tabId}`)).toHaveAttribute(
      'data-panel-open',
      'true'
    );
    expect(mountCounts.get(tabId)).toBe(1);
    expect(unmountCounts.get(tabId) ?? 0).toBe(0);
    // Still only one tab in the document — not a second "Terminal 1".
    expect(screen.getAllByTestId(/terminal-tab-view-/)).toHaveLength(1);
  });

  it('does not wipe persisted tab ids when toggling open/closed', async () => {
    renderPanel();

    const tabId = await openViaShortcut();

    await act(async () => {
      emit('toggle-terminal'); // close
    });
    await act(async () => {
      emit('toggle-terminal'); // reopen
    });

    const stored = loadTerminalState(sessionId);
    expect(stored.tabs.map((t) => t.id)).toEqual([tabId]);
    expect(stored.activeTabId).toBe(tabId);
    expect(stored.open).toBe(true);
  });

  it('collapses with width 0 instead of unmounting (display:none / hidden)', async () => {
    const { container } = renderPanel();
    await openViaShortcut();

    await act(async () => {
      emit('toggle-terminal');
    });

    const aside = container.querySelector('aside');
    expect(aside).not.toBeNull();
    expect(aside).not.toHaveClass('hidden');
    expect(aside).toHaveStyle({ width: '0px' });
    expect(aside).toHaveAttribute('aria-hidden', 'true');
  });
});
