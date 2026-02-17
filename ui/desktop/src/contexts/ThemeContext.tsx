import React, { createContext, useContext, useEffect, useState, useCallback } from 'react';
import {
  buildMcpHostStyles,
  lightTokens,
  darkTokens,
} from '../theme/theme-tokens';
import {
  getActiveThemeId,
  setActiveThemeId,
  getThemePreset,
  resolvePresetTokens,
} from '../themes/presets';
import type { ThemePreset } from '../themes/presets/types';
import type { McpUiHostStyles } from '@modelcontextprotocol/ext-apps/app-bridge';

type ThemePreference = 'light' | 'dark' | 'system';
type ResolvedTheme = 'light' | 'dark';

interface ThemeContextValue {
  userThemePreference: ThemePreference;
  setUserThemePreference: (pref: ThemePreference) => void;
  resolvedTheme: ResolvedTheme;
  mcpHostStyles: McpUiHostStyles;
  refreshTokens: () => void;
  activePresetId: string | null;
  applyPreset: (presetId: string | null) => void;
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

/**
 * Apply resolved tokens to :root. If a preset is active its color overrides
 * are merged on top of the goose-classic defaults. Any per-key localStorage
 * overrides (`theme-overrides`) are applied last.
 */
function applyResolvedTokens(theme: ResolvedTheme, preset: ThemePreset | undefined): void {
  const root = document.documentElement;

  // Start with full defaults or preset-merged tokens
  const tokens = preset ? resolvePresetTokens(preset, theme) : (theme === 'dark' ? darkTokens : lightTokens);

  // Layer any per-key localStorage overrides on top
  let merged = { ...tokens };
  const stored = localStorage.getItem('theme-overrides');
  if (stored) {
    try {
      const overrides = JSON.parse(stored);
      merged = { ...merged, ...(overrides[theme] ?? {}) };
    } catch {
      // ignore bad JSON
    }
  }

  for (const [key, value] of Object.entries(merged)) {
    root.style.setProperty(key, value as string);
  }
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
  const [activePresetId, setActivePresetIdState] = useState<string | null>(
    () => getActiveThemeId()
  );
  // Bumped to force re-application of tokens (e.g., after custom color save)
  const [tokenVersion, setTokenVersion] = useState(0);

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
    const handleChange = () => setResolvedTheme(getSystemTheme());

    mediaQuery.addEventListener('change', handleChange);
    return () => mediaQuery.removeEventListener('change', handleChange);
  }, [userThemePreference]);

  // Re-read localStorage overrides and re-apply tokens in this window
  const refreshTokens = useCallback(() => {
    setTokenVersion((v) => v + 1);
    window.electron?.broadcastThemeChange({
      mode: resolvedTheme,
      useSystemTheme: userThemePreference === 'system',
      theme: resolvedTheme,
      tokensUpdated: true,
    });
  }, [resolvedTheme, userThemePreference]);

  // Apply a preset (or null to reset to goose-classic defaults)
  const applyPreset = useCallback(
    (presetId: string | null) => {
      setActiveThemeId(presetId);
      setActivePresetIdState(presetId);
      setTokenVersion((v) => v + 1);

      window.electron?.broadcastThemeChange({
        mode: resolvedTheme,
        useSystemTheme: userThemePreference === 'system',
        theme: resolvedTheme,
        tokensUpdated: true,
      });
    },
    [resolvedTheme, userThemePreference]
  );

  // Listen for theme changes from other windows (via Electron IPC)
  useEffect(() => {
    if (!window.electron) return;

    const handleThemeChanged = (_event: unknown, ...args: unknown[]) => {
      const themeData = args[0] as {
        useSystemTheme: boolean;
        theme: string;
        tokensUpdated?: boolean;
      };
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

      if (themeData.tokensUpdated) {
        // Re-read active preset from localStorage (may have changed in another window)
        setActivePresetIdState(getActiveThemeId());
        setTokenVersion((v) => v + 1);
      }
    };

    window.electron.on('theme-changed', handleThemeChanged);
    return () => {
      window.electron.off('theme-changed', handleThemeChanged);
    };
  }, []);

  // Apply theme class and CSS tokens whenever resolvedTheme or tokens change
  useEffect(() => {
    applyThemeToDocument(resolvedTheme);
    const preset = activePresetId ? getThemePreset(activePresetId) : undefined;
    applyResolvedTokens(resolvedTheme, preset);
  }, [resolvedTheme, activePresetId, tokenVersion]);

  const value: ThemeContextValue = {
    userThemePreference,
    setUserThemePreference,
    resolvedTheme,
    mcpHostStyles,
    refreshTokens,
    activePresetId,
    applyPreset,
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
