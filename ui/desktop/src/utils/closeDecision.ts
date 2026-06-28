import type { CloseAction } from './settings';

/**
 * The next step the renderer should take when the main window is asked to close.
 *
 * - `show-choice`   — ask the user (tray vs quit) before doing anything.
 * - `confirm-busy`  — about to quit while a session is busy; confirm first.
 * - `hide-to-tray`  — resolved: minimize the window to the system tray.
 * - `quit`          — resolved: close the window.
 * - `abort`         — do nothing; keep the window open.
 */
export type CloseOutcome = 'show-choice' | 'confirm-busy' | 'hide-to-tray' | 'quit' | 'abort';

/**
 * Resolve a concrete close action (chosen explicitly or remembered) against the current
 * busy state. Quitting while busy always routes through a confirmation so an in-flight
 * turn is never discarded silently — independent of whether the user was asked.
 */
export function resolveCloseOutcome(action: 'tray' | 'quit', isBusy: boolean): CloseOutcome {
  if (action === 'tray') {
    return 'hide-to-tray';
  }
  return isBusy ? 'confirm-busy' : 'quit';
}

/**
 * Decide the first step from the remembered `closeAction` setting. `ask` shows the choice
 * modal; a remembered `tray`/`quit` skips it but still honors the busy confirmation.
 */
export function decideInitialStep(closeAction: CloseAction, isBusy: boolean): CloseOutcome {
  if (closeAction === 'ask') {
    return 'show-choice';
  }
  return resolveCloseOutcome(closeAction, isBusy);
}
