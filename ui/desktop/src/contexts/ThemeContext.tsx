import React, { createContext, useContext, useEffect, useState, useCallback } from 'react';
import { getThemeVariables } from '../api';

type ThemePreference = 'light' | 'dark' | 'system';
type ResolvedTheme = 'light' | 'dark';

interface ThemeContextValue {
  userThemePreference: ThemePreference;
  setUserThemePreference: (pref: ThemePreference) => void;
  resolvedTheme: ResolvedTheme;
  themeVariables: Record<string, string> | null;
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

function loadThemePreference(): ThemePreference {
  const useSystemTheme = localStorage.getItem('use_system_theme');
  if (useSystemTheme === 'true') {
    return 'system';
  }

  const savedTheme = localStorage.getItem('theme');
  if (savedTheme === 'dark') {
    return 'dark';
  }

  return 'light';
}

function saveThemePreference(preference: ThemePreference): void {
  if (preference === 'system') {
    localStorage.setItem('use_system_theme', 'true');
  } else {
    localStorage.setItem('use_system_theme', 'false');
    localStorage.setItem('theme', preference);
  }
}

function applyThemeToDocument(theme: ResolvedTheme): void {
  const toRemove = theme === 'dark' ? 'light' : 'dark';
  document.documentElement.classList.add(theme);
  document.documentElement.classList.remove(toRemove);
}

const THEME_STYLE_ID = 'goose-mcp-theme';

/**
 * Parse light-dark() values and generate CSS for :root and .dark
 * Example: "light-dark(#fff, #000)" => { light: "#fff", dark: "#000" }
 */
function parseLightDark(value: string): { light: string; dark: string } | null {
  const match = value.match(/^light-dark\((.+),\s*(.+)\)$/);
  if (!match) return null;
  return { light: match[1].trim(), dark: match[2].trim() };
}

/**
 * Generate and inject CSS from theme variables
 */
function injectThemeCSS(variables: Record<string, string> | null): void {
  const styleElement = document.getElementById(THEME_STYLE_ID) as HTMLStyleElement;

  if (!variables || Object.keys(variables).length === 0) {
    // Remove style element if no variables
    if (styleElement) {
      styleElement.remove();
    }
    return;
  }

  // Separate variables into light and dark mode
  const rootVars: string[] = [];
  const darkVars: string[] = [];

  for (const [name, value] of Object.entries(variables)) {
    const parsed = parseLightDark(value);
    if (parsed) {
      rootVars.push(`  ${name}: ${parsed.light};`);
      darkVars.push(`  ${name}: ${parsed.dark};`);
    }
  }

  // Generate CSS
  const css = `:root {\n${rootVars.join('\n')}\n}\n\n.dark {\n${darkVars.join('\n')}\n}`;

  // Inject or update the style tag
  if (!styleElement) {
    const newStyleElement = document.createElement('style');
    newStyleElement.id = THEME_STYLE_ID;
    newStyleElement.textContent = css;
    document.head.appendChild(newStyleElement);
  } else {
    styleElement.textContent = css;
  }
}

async function loadThemeVariables(): Promise<Record<string, string> | null> {
  try {
    const response = await getThemeVariables();
    return response.data?.variables || null;
  } catch (err) {
    console.warn('Failed to load theme variables:', err);
    return null;
  }
}

interface ThemeProviderProps {
  children: React.ReactNode;
}

export function ThemeProvider({ children }: ThemeProviderProps) {
  const [userThemePreference, setUserThemePreferenceState] =
    useState<ThemePreference>(loadThemePreference);
  const [resolvedTheme, setResolvedTheme] = useState<ResolvedTheme>(() =>
    resolveTheme(loadThemePreference())
  );
  const [themeVariables, setThemeVariables] = useState<Record<string, string> | null>(null);

  const setUserThemePreference = useCallback((preference: ThemePreference) => {
    setUserThemePreferenceState(preference);
    saveThemePreference(preference);

    const resolved = resolveTheme(preference);
    setResolvedTheme(resolved);

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
      saveThemePreference(newPreference);
      setResolvedTheme(resolveTheme(newPreference));
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

  // Load theme variables and inject CSS on mount
  useEffect(() => {
    loadThemeVariables().then((variables) => {
      setThemeVariables(variables);
      injectThemeCSS(variables);
    });
  }, []);

  const value: ThemeContextValue = {
    userThemePreference,
    setUserThemePreference,
    resolvedTheme,
    themeVariables,
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
