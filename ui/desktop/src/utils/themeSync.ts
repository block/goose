/**
 * Theme synchronization utility to handle theme state across multiple Goose windows
 */

export interface ThemeState {
  themeMode: 'light' | 'dark' | 'system';
  isDarkMode: boolean;
  timestamp: number;
}

const THEME_SYNC_KEY = 'goose_theme_sync';
const THEME_SYNC_EVENT = 'goose-theme-changed';

/**
 * Get the current theme state from localStorage
 */
export function getCurrentThemeState(): ThemeState {
  const useSystemTheme = localStorage.getItem('use_system_theme') === 'true';
  const savedTheme = localStorage.getItem('theme');
  const systemPrefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;

  const themeMode: 'light' | 'dark' | 'system' = useSystemTheme
    ? 'system'
    : savedTheme === 'dark'
      ? 'dark'
      : 'light';

  const isDarkMode = useSystemTheme ? systemPrefersDark : savedTheme === 'dark';

  return {
    themeMode,
    isDarkMode,
    timestamp: Date.now(),
  };
}

/**
 * Apply theme classes to the document
 */
export function applyTheme(isDarkMode: boolean): void {
  if (isDarkMode) {
    document.documentElement.classList.add('dark');
    document.documentElement.classList.remove('light');
  } else {
    document.documentElement.classList.remove('dark');
    document.documentElement.classList.add('light');
  }
}

/**
 * Broadcast theme change to other windows
 */
export function broadcastThemeChange(themeState: ThemeState): void {
  try {
    // Store the current state for other windows to read
    localStorage.setItem(THEME_SYNC_KEY, JSON.stringify(themeState));
    window.dispatchEvent(
      new CustomEvent(THEME_SYNC_EVENT, {
        detail: themeState,
      })
    );
  } catch (error) {
    console.warn('[Theme Sync] Failed to broadcast theme change:', error);
  }
}

/**
 * Listen for theme changes from other windows
 */
export function listenForThemeChanges(callback: (themeState: ThemeState) => void): () => void {
  const handleStorageChange = (event: globalThis.StorageEvent) => {
    if (event.key === THEME_SYNC_KEY && event.newValue) {
      try {
        const themeState: ThemeState = JSON.parse(event.newValue);
        callback(themeState);
      } catch (error) {
        console.warn('[Theme Sync] Failed to parse theme state from storage:', error);
      }
    }
  };

  const handleCustomEvent = (event: globalThis.CustomEvent<ThemeState>) => {
    callback(event.detail);
  };

  // Listen for storage changes (cross-window)
  window.addEventListener('storage', handleStorageChange);

  // Listen for custom events (same-window)
  window.addEventListener(THEME_SYNC_EVENT, handleCustomEvent as globalThis.EventListener);

  // Return cleanup function
  return () => {
    window.removeEventListener('storage', handleStorageChange);
    window.removeEventListener(THEME_SYNC_EVENT, handleCustomEvent as globalThis.EventListener);
  };
}

/**
 * Initialize theme synchronization
 */
export function initializeThemeSync(onThemeChange?: (themeState: ThemeState) => void): () => void {
  // Apply current theme immediately
  const currentState = getCurrentThemeState();
  applyTheme(currentState.isDarkMode);

  // Set up listener for theme changes from other windows
  const cleanup = listenForThemeChanges((themeState) => {
    console.log('[Theme Sync] Received theme change from another window:', themeState);

    applyTheme(themeState.isDarkMode);

    // Update localStorage to match
    if (themeState.themeMode === 'system') {
      localStorage.setItem('use_system_theme', 'true');
    } else {
      localStorage.setItem('use_system_theme', 'false');
      localStorage.setItem('theme', themeState.themeMode);
    }

    // Notify callback if provided
    if (onThemeChange) {
      onThemeChange(themeState);
    }
  });

  console.log('[Theme Sync] Initialized theme synchronization');
  return cleanup;
}
