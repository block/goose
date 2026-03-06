function isMac(): boolean {
  return window.electron?.platform === 'darwin';
}

export function getNavigationShortcutKeys(): string {
  return isMac() ? '⌘↑/⌘↓' : 'Ctrl+↑/Ctrl+↓';
}

export function getSearchShortcutText(): string {
  return isMac() ? '⌘F' : 'Ctrl+F';
}
