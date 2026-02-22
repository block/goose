import React, { createContext, useContext, useEffect, useState, useCallback } from 'react';

type ThemePreference = 'light' | 'dark' | 'system';
type ResolvedTheme = 'light' | 'dark';

interface ThemeContextValue {
  userThemePreference: ThemePreference;
  setUserThemePreference: (pref: ThemePreference) => void;
  resolvedTheme: ResolvedTheme;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

function getSystemTheme(): ResolvedTheme {
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

function resolveTheme(preference: ThemePreference): ResolvedTheme {
  if (preference === 'system') {
    return getSystemTheme();
  }
  return preference;
}

function applyThemeToDocument(theme: ResolvedTheme): void {
  const toRemove = theme === 'dark' ? 'light' : 'dark';
  document.documentElement.classList.add(theme);
  document.documentElement.classList.remove(toRemove);
}

interface ThemeProviderProps {
  children: React.ReactNode;
}

export function ThemeProvider({ children }: ThemeProviderProps) {
  // Start with light theme to avoid flash, will update once settings load
  const [userThemePreference, setUserThemePreferenceState] = useState<ThemePreference>('light');
  const [resolvedTheme, setResolvedTheme] = useState<ResolvedTheme>('light');

  useEffect(() => {
    async function loadThemeFromSettings() {
      try {
        const [useSystemTheme, savedTheme] = await Promise.all([
          window.electron.getSetting('useSystemTheme'),
          window.electron.getSetting('theme'),
        ]);

        let preference: ThemePreference;
        if (useSystemTheme) {
          preference = 'system';
        } else {
          preference = savedTheme;
        }

        setUserThemePreferenceState(preference);
        setResolvedTheme(resolveTheme(preference));
      } catch (error) {
        console.warn('[ThemeContext] Failed to load theme settings:', error);
      }
    }

    loadThemeFromSettings();
  }, []);

  const setUserThemePreference = useCallback(async (preference: ThemePreference) => {
    setUserThemePreferenceState(preference);

    const resolved = resolveTheme(preference);
    setResolvedTheme(resolved);

    // Save to settings
    try {
      if (preference === 'system') {
        await window.electron.setSetting('useSystemTheme', true);
      } else {
        await window.electron.setSetting('useSystemTheme', false);
        await window.electron.setSetting('theme', preference);
      }
    } catch (error) {
      console.warn('[ThemeContext] Failed to save theme settings:', error);
    }

    // Broadcast to other windows via Electron
    window.electron?.broadcastThemeChange({
      mode: resolved,
      useSystemTheme: preference === 'system',
      theme: resolved,
    });
  }, []);

  // Listen for system theme changes when preference is 'system'
  useEffect(() => {
    if (userThemePreference !== 'system') return;

    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');

    const handleChange = () => {
      setResolvedTheme(getSystemTheme());
    };

    mediaQuery.addEventListener('change', handleChange);
    return () => mediaQuery.removeEventListener('change', handleChange);
  }, [userThemePreference]);

  // Listen for theme changes from other windows (via Electron IPC)
  useEffect(() => {
    if (!window.electron) return;

    const handleThemeChanged = (_event: unknown, ...args: unknown[]) => {
      const themeData = args[0] as { useSystemTheme: boolean; theme: string };
      const newPreference: ThemePreference = themeData.useSystemTheme
        ? 'system'
        : themeData.theme === 'dark'
          ? 'dark'
          : 'light';

      setUserThemePreferenceState(newPreference);
      setResolvedTheme(resolveTheme(newPreference));

      // Save to settings (don't await, fire and forget)
      if (newPreference === 'system') {
        window.electron.setSetting('useSystemTheme', true);
      } else {
        window.electron.setSetting('useSystemTheme', false);
        window.electron.setSetting('theme', newPreference);
      }
    };

    window.electron.on('theme-changed', handleThemeChanged);
    return () => {
      window.electron.off('theme-changed', handleThemeChanged);
    };
  }, []);

  // Apply theme to document whenever resolvedTheme changes
  useEffect(() => {
    applyThemeToDocument(resolvedTheme);
  }, [resolvedTheme]);

  const value: ThemeContextValue = {
    userThemePreference,
    setUserThemePreference,
    resolvedTheme,
  };

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme(): ThemeContextValue {
  const context = useContext(ThemeContext);
  if (!context) {
    throw new Error('useTheme must be used within a ThemeProvider');
  }
  return context;
}
