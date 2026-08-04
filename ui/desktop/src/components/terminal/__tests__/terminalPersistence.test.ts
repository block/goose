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
});
