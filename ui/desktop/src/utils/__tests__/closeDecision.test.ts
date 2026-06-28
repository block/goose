import { describe, it, expect } from 'vitest';
import { decideInitialStep, resolveCloseOutcome } from '../closeDecision';

describe('decideInitialStep', () => {
  it('asks the user when the remembered action is "ask"', () => {
    expect(decideInitialStep('ask', false)).toBe('show-choice');
    expect(decideInitialStep('ask', true)).toBe('show-choice');
  });

  it('hides to tray for a remembered "tray" action regardless of busy state', () => {
    expect(decideInitialStep('tray', false)).toBe('hide-to-tray');
    expect(decideInitialStep('tray', true)).toBe('hide-to-tray');
  });

  it('quits immediately for a remembered "quit" action when idle', () => {
    expect(decideInitialStep('quit', false)).toBe('quit');
  });

  it('still confirms when a remembered "quit" action would discard an in-flight turn', () => {
    // Regression guard: a remembered quit must never silently kill a busy session.
    expect(decideInitialStep('quit', true)).toBe('confirm-busy');
  });
});

describe('resolveCloseOutcome', () => {
  it('maps tray to hide-to-tray (busy is irrelevant)', () => {
    expect(resolveCloseOutcome('tray', false)).toBe('hide-to-tray');
    expect(resolveCloseOutcome('tray', true)).toBe('hide-to-tray');
  });

  it('quits directly when not busy', () => {
    expect(resolveCloseOutcome('quit', false)).toBe('quit');
  });

  it('routes to a busy confirmation when busy', () => {
    expect(resolveCloseOutcome('quit', true)).toBe('confirm-busy');
  });
});
