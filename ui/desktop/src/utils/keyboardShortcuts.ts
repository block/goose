import { platform } from '../platform';

function isMac(): boolean {
  return platform.platform === 'darwin';
}

export function getNavigationShortcutText(): string {
  return isMac() ? '⌘↑/⌘↓ to navigate messages' : 'Ctrl+↑/Ctrl+↓ to navigate messages';
}

export function getSearchShortcutText(): string {
  return isMac() ? '⌘F' : 'Ctrl+F';
}
