function isMac(): boolean {
  return window.electron?.platform === 'darwin';
}

export function getNavigationShortcutText(): string {
  return isMac() ? '⌘↑/⌘↓ to navigate messages' : 'Ctrl+↑/Ctrl+↓ to navigate messages';
}

export function getSearchShortcutText(): string {
  return isMac() ? '⌘F' : 'Ctrl+F';
}
