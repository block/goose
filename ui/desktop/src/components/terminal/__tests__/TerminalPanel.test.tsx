import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
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

  it('closes a tab with × and collapses the whole section when it is the last tab', async () => {
    const onRequestChatFocus = vi.fn();
    const user = userEvent.setup();
    const { container } = render(
      <IntlTestWrapper>
        <TerminalPanel
          sessionId={sessionId}
          cwd="/tmp/goose"
          isActiveSession
          onRequestChatFocus={onRequestChatFocus}
        />
      </IntlTestWrapper>
    );

    const tabId = await openViaShortcut();
    expect(container.querySelector('aside')).not.toBeNull();

    await user.click(screen.getByTestId(`terminal-close-tab-${tabId}`));

    expect(window.electron.terminalKill).toHaveBeenCalledWith({
      sessionId,
      tabId,
    });
    expect(screen.queryByTestId(`terminal-tab-view-${tabId}`)).not.toBeInTheDocument();
    expect(container.querySelector('aside')).toBeNull();
    expect(onRequestChatFocus).toHaveBeenCalled();

    const stored = loadTerminalState(sessionId);
    expect(stored.open).toBe(false);
    expect(stored.tabs).toEqual([]);
    expect(stored.activeTabId).toBeNull();

    // Must not immediately spawn a replacement tab after closing the last one.
    expect(screen.queryByTestId(/terminal-tab-view-/)).not.toBeInTheDocument();
  });

  it('keeps the panel open when closing a non-last tab', async () => {
    const user = userEvent.setup();
    const { container } = renderPanel();

    const firstTabId = await openViaShortcut();
    await user.click(screen.getByTestId('terminal-new-tab'));

    const views = await waitFor(() => screen.getAllByTestId(/terminal-tab-view-/));
    expect(views).toHaveLength(2);
    const secondTabId = views
      .map((node) => node.getAttribute('data-tab-id')!)
      .find((id) => id !== firstTabId)!;

    await user.click(screen.getByTestId(`terminal-close-tab-${firstTabId}`));

    expect(window.electron.terminalKill).toHaveBeenCalledWith({
      sessionId,
      tabId: firstTabId,
    });
    expect(screen.queryByTestId(`terminal-tab-view-${firstTabId}`)).not.toBeInTheDocument();
    expect(screen.getByTestId(`terminal-tab-view-${secondTabId}`)).toBeInTheDocument();
    expect(container.querySelector('aside')).not.toBeNull();
    expect(container.querySelector('aside')).toHaveAttribute('aria-hidden', 'false');

    const stored = loadTerminalState(sessionId);
    expect(stored.open).toBe(true);
    expect(stored.tabs.map((t) => t.id)).toEqual([secondTabId]);
  });

  it('places the new-tab control immediately after the newest tab, not pinned to the pane edge', async () => {
    const user = userEvent.setup();
    renderPanel();

    const firstTabId = await openViaShortcut();
    await user.click(screen.getByTestId('terminal-new-tab'));

    const views = await waitFor(() => screen.getAllByTestId(/terminal-tab-view-/));
    expect(views).toHaveLength(2);
    const secondTabId = views
      .map((node) => node.getAttribute('data-tab-id')!)
      .find((id) => id !== firstTabId)!;

    const strip = screen.getByTestId('terminal-tab-strip');
    const stripChildren = Array.from(strip.children);
    const tabIndexes = stripChildren.map((el) => el.getAttribute('data-testid'));

    expect(tabIndexes).toEqual([
      `terminal-tab-${firstTabId}`,
      `terminal-tab-${secondTabId}`,
      'terminal-new-tab',
    ]);

    // + is a sibling after tabs in the same strip — not a separate flex-end pin.
    expect(strip.querySelector('[data-testid="terminal-new-tab"]')?.parentElement).toBe(strip);
    expect(screen.getByTestId('terminal-tab-strip').className).toContain('z-30');
    expect(screen.getByTestId('terminal-content').className).toContain('z-0');
  });

  it('new-tab and close-tab buttons receive clicks above the terminal content layer', async () => {
    const user = userEvent.setup();
    renderPanel();

    const firstTabId = await openViaShortcut();

    // Click + must add a second tab (regression: xterm overlay ate the click).
    await user.click(screen.getByTestId('terminal-new-tab'));
    await waitFor(() => {
      expect(screen.getAllByTestId(/terminal-tab-view-/)).toHaveLength(2);
    });

    const secondTabId = screen
      .getAllByTestId(/terminal-tab-view-/)
      .map((node) => node.getAttribute('data-tab-id')!)
      .find((id) => id !== firstTabId)!;

    // Click × on the active/newest tab must close that tab only.
    await user.click(screen.getByTestId(`terminal-close-tab-${secondTabId}`));
    expect(window.electron.terminalKill).toHaveBeenCalledWith({
      sessionId,
      tabId: secondTabId,
    });
    expect(screen.queryByTestId(`terminal-tab-view-${secondTabId}`)).not.toBeInTheDocument();
    expect(screen.getByTestId(`terminal-tab-view-${firstTabId}`)).toBeInTheDocument();
  });
});
