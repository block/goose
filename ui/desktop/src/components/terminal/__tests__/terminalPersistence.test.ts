import { afterEach, describe, expect, it } from 'vitest';
import {
  TERMINAL_DEFAULT_WIDTH,
  clearTerminalState,
  loadTerminalState,
  saveTerminalState,
} from '../terminalPersistence';

describe('terminalPersistence', () => {
  const sessionId = 'session-test-1';

  afterEach(() => {
    clearTerminalState(sessionId);
  });

  it('returns defaults when nothing is stored', () => {
    expect(loadTerminalState(sessionId)).toEqual({
      open: false,
      width: TERMINAL_DEFAULT_WIDTH,
      tabs: [],
      activeTabId: null,
    });
  });

  it('round-trips open, width, and tabs', () => {
    saveTerminalState(sessionId, {
      open: true,
      width: 480,
      tabs: [
        { id: 'a', title: '1' },
        { id: 'b', title: '2' },
      ],
      activeTabId: 'b',
    });

    expect(loadTerminalState(sessionId)).toEqual({
      open: true,
      width: 480,
      tabs: [
        { id: 'a', title: '1' },
        { id: 'b', title: '2' },
      ],
      activeTabId: 'b',
    });
  });

  it('reproduces the stale persist wipe that forced a new PTY on reopen', () => {
    // Regression: open/close used a stale closure that wrote open:true with tabs:[].
    // Reloading then called ensureTab() and spawned a brand-new terminal id.
    saveTerminalState(sessionId, {
      open: true,
      width: 420,
      tabs: [{ id: 'term_original', title: '1' }],
      activeTabId: 'term_original',
    });

    saveTerminalState(sessionId, {
      open: true,
      width: 420,
      tabs: [],
      activeTabId: null,
    });

    const wiped = loadTerminalState(sessionId);
    expect(wiped.tabs).toEqual([]);
    expect(wiped.activeTabId).toBeNull();
  });
});
