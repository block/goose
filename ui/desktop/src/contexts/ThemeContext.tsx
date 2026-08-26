import React, { createContext, useContext, useEffect, useState, useCallback, useMemo } from 'react';
import { applyThemeTokens, buildMcpHostStyles, themes } from '../theme/theme-tokens';
import {
  DEFAULT_CATPPUCCIN_ACCENT,
  isCatppuccinAccent,
  isThemeId,
  isThemePreference,
  type CatppuccinAccent,
  type ThemeId,
  type ThemePreference,
  type ThemeVariant,
} from '../theme/types';
import type { McpUiHostStyles } from '@modelcontextprotocol/ext-apps/app-bridge';

interface ThemeContextValue {
  userThemePreference: ThemePreference;
  setUserThemePreference: (pref: ThemePreference) => void;
  resolvedThemeId: ThemeId;
  resolvedTheme: ThemeVariant;
  catppuccinAccent: CatppuccinAccent;
  setCatppuccinAccent: (accent: CatppuccinAccent) => void;
  mcpHostStyles: McpUiHostStyles;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

function getSystemTheme(): ThemeVariant {
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

// Resolve a user preference to a concrete theme id. 'system' picks the light or
// dark built-in from the OS; named themes map to themselves.
function resolveThemeId(preference: ThemePreference): ThemeId {
  if (preference === 'system') {
    return getSystemTheme();
  }
  return preference;
}

function applyThemeToDocument(theme: ThemeVariant): void {
  const toRemove = theme === 'dark' ? 'light' : 'dark';
  document.documentElement.classList.add(theme);
  document.documentElement.classList.remove(toRemove);
  document.documentElement.style.colorScheme = theme;
}

interface ThemeProviderProps {
  children: React.ReactNode;
}

export function ThemeProvider({ children }: ThemeProviderProps) {
  // Start with light theme to avoid flash, will update once settings load
  const [userThemePreference, setUserThemePreferenceState] = useState<ThemePreference>('light');
  const [resolvedThemeId, setResolvedThemeId] = useState<ThemeId>('light');
  const [catppuccinAccent, setCatppuccinAccentState] =
    useState<CatppuccinAccent>(DEFAULT_CATPPUCCIN_ACCENT);
  const resolvedTheme = themes[resolvedThemeId].variant;
  const mcpHostStyles = useMemo(
    () => buildMcpHostStyles(resolvedThemeId, catppuccinAccent),
    [resolvedThemeId, catppuccinAccent]
  );

  useEffect(() => {
    async function loadThemeFromSettings() {
      try {
        const [useSystemTheme, savedTheme, savedAccent] = await Promise.all([
          window.electron.getSetting('useSystemTheme'),
          window.electron.getSetting('theme'),
          window.electron.getSetting('catppuccinAccent'),
        ]);

        const preference: ThemePreference = useSystemTheme
          ? 'system'
          : isThemePreference(savedTheme)
            ? savedTheme
            : 'light';

        setUserThemePreferenceState(preference);
        setResolvedThemeId(resolveThemeId(preference));
        setCatppuccinAccentState(
          isCatppuccinAccent(savedAccent) ? savedAccent : DEFAULT_CATPPUCCIN_ACCENT
        );
      } catch (error) {
        console.warn('[ThemeContext] Failed to load theme settings:', error);
      }
    }

    loadThemeFromSettings();
  }, []);

  const setUserThemePreference = useCallback(async (preference: ThemePreference) => {
    setUserThemePreferenceState(preference);

    const resolvedId = resolveThemeId(preference);
    setResolvedThemeId(resolvedId);

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

    // Theme-only: a newer accent can save+broadcast while these writes are in flight.
    window.electron?.broadcastThemeChange({
      mode: themes[resolvedId].variant,
      useSystemTheme: preference === 'system',
      theme: resolvedId,
    });
  }, []);

  const setCatppuccinAccent = useCallback(async (accent: CatppuccinAccent) => {
    setCatppuccinAccentState(accent);

    try {
      await window.electron.setSetting('catppuccinAccent', accent);
    } catch (error) {
      console.warn('[ThemeContext] Failed to save Catppuccin accent:', error);
    }

    window.electron?.broadcastThemeChange({
      catppuccinAccent: accent,
    });
  }, []);

  // Listen for system theme changes when preference is 'system'
  useEffect(() => {
    if (userThemePreference !== 'system') return;

    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');

    const handleChange = () => {
      setResolvedThemeId(getSystemTheme());
    };

    mediaQuery.addEventListener('change', handleChange);
    return () => mediaQuery.removeEventListener('change', handleChange);
  }, [userThemePreference]);

  // Listen for theme changes from other windows (via Electron IPC)
  useEffect(() => {
    if (!window.electron) return;

    const handleThemeChanged = (_event: unknown, ...args: unknown[]) => {
      const themeData = args[0] as {
        useSystemTheme?: boolean;
        theme?: ThemeId;
        catppuccinAccent?: CatppuccinAccent;
      };
      const hasThemeUpdate =
        typeof themeData.useSystemTheme === 'boolean' || isThemeId(themeData.theme);

      if (hasThemeUpdate) {
        const newPreference: ThemePreference = themeData.useSystemTheme
          ? 'system'
          : isThemeId(themeData.theme)
            ? themeData.theme
            : 'light';

        setUserThemePreferenceState(newPreference);
        setResolvedThemeId(resolveThemeId(newPreference));

        if (newPreference === 'system') {
          window.electron.setSetting('useSystemTheme', true);
        } else {
          window.electron.setSetting('useSystemTheme', false);
          window.electron.setSetting('theme', newPreference);
        }
      }

      if (isCatppuccinAccent(themeData.catppuccinAccent)) {
        setCatppuccinAccentState(themeData.catppuccinAccent);
        window.electron.setSetting('catppuccinAccent', themeData.catppuccinAccent);
      }
    };

    window.electron.on('theme-changed', handleThemeChanged);
    return () => {
      window.electron.off('theme-changed', handleThemeChanged);
    };
  }, []);

  // Apply theme class and CSS tokens whenever the resolved theme or accent changes
  useEffect(() => {
    applyThemeToDocument(themes[resolvedThemeId].variant);
    applyThemeTokens(resolvedThemeId, catppuccinAccent);
    document.documentElement.dataset.theme = resolvedThemeId;
    document.documentElement.dataset.catppuccinAccent = catppuccinAccent;
  }, [resolvedThemeId, catppuccinAccent]);

  const value: ThemeContextValue = {
    userThemePreference,
    setUserThemePreference,
    resolvedThemeId,
    resolvedTheme,
    catppuccinAccent,
    setCatppuccinAccent,
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
