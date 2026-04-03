import React, { createContext, useContext, useEffect, useState, useCallback } from 'react';
import { applyThemeTokens, buildMcpHostStyles } from '../theme/theme-tokens';
import type { McpUiHostStyles } from '@modelcontextprotocol/ext-apps/app-bridge';
import { platform } from '../platform';

type ThemePreference = 'light' | 'dark' | 'system';
type ResolvedTheme = 'light' | 'dark';

interface ThemeContextValue {
  userThemePreference: ThemePreference;
  setUserThemePreference: (pref: ThemePreference) => void;
  resolvedTheme: ResolvedTheme;
  mcpHostStyles: McpUiHostStyles;
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
  document.documentElement.style.colorScheme = theme;
}

// Built once — light-dark() values are theme-independent
const mcpHostStyles = buildMcpHostStyles();

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
          platform.getSetting('useSystemTheme'),
          platform.getSetting('theme'),
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
        await platform.setSetting('useSystemTheme', true);
      } else {
        await platform.setSetting('useSystemTheme', false);
        await platform.setSetting('theme', preference);
      }
    } catch (error) {
      console.warn('[ThemeContext] Failed to save theme settings:', error);
    }

    // Broadcast to other windows via Electron
    platform?.broadcastThemeChange({
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
    if (!platform) return;

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
        platform.setSetting('useSystemTheme', true);
      } else {
        platform.setSetting('useSystemTheme', false);
        platform.setSetting('theme', newPreference);
      }
    };

    platform.on('theme-changed', handleThemeChanged);
    return () => {
      platform.off('theme-changed', handleThemeChanged);
    };
  }, []);

  // Apply theme class and CSS tokens whenever resolvedTheme changes
  useEffect(() => {
    applyThemeToDocument(resolvedTheme);
    applyThemeTokens(resolvedTheme);
  }, [resolvedTheme]);

  const value: ThemeContextValue = {
    userThemePreference,
    setUserThemePreference,
    resolvedTheme,
    mcpHostStyles,
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
