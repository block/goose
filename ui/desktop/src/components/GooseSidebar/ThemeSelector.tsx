import React, { useEffect, useState } from 'react';
import { Moon, Sliders, Sun } from 'lucide-react';
import { Button } from '../ui/button';

interface ThemeSelectorProps {
  className?: string;
  hideTitle?: boolean;
  horizontal?: boolean;
}

const getIsDarkMode = (mode: 'light' | 'dark' | 'system'): boolean => {
  if (mode === 'system') {
    return window.matchMedia('(prefers-color-scheme: dark)').matches;
  }
  return mode === 'dark';
};

const getThemeMode = (): 'light' | 'dark' | 'system' => {
  const savedUseSystemTheme = localStorage.getItem('use_system_theme');
  if (savedUseSystemTheme === 'true') {
    return 'system';
  }

  const savedTheme = localStorage.getItem('theme');
  if (savedTheme) {
    return savedTheme === 'dark' ? 'dark' : 'light';
  }

  return getIsDarkMode('system') ? 'dark' : 'light';
};

const setThemeModeStorage = (mode: 'light' | 'dark' | 'system') => {
  if (mode === 'system') {
    localStorage.setItem('use_system_theme', 'true');
  } else {
    localStorage.setItem('use_system_theme', 'false');
    localStorage.setItem('theme', mode);
  }

  const themeData = {
    mode,
    useSystemTheme: mode === 'system',
    theme: mode === 'system' ? '' : mode,
  };

  window.electron?.broadcastThemeChange(themeData);
};

const ThemeSelector: React.FC<ThemeSelectorProps> = ({
  className = '',
  hideTitle = false,
  horizontal = false,
}) => {
  const [themeMode, setThemeMode] = useState<'light' | 'dark' | 'system'>(getThemeMode);
  const [isDarkMode, setDarkMode] = useState(() => getIsDarkMode(getThemeMode()));

  useEffect(() => {
    const handleStorageChange = (e: { key: string | null; newValue: string | null }) => {
      if (e.key === 'use_system_theme' || e.key === 'theme') {
        const newThemeMode = getThemeMode();
        setThemeMode(newThemeMode);
        setDarkMode(getIsDarkMode(newThemeMode));
      }
    };

    window.addEventListener('storage', handleStorageChange);

    return () => {
      window.removeEventListener('storage', handleStorageChange);
    };
  }, []);

  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');

    const handleThemeChange = (e: { matches: boolean }) => {
      if (themeMode === 'system') {
        setDarkMode(e.matches);
      }
    };

    mediaQuery.addEventListener('change', handleThemeChange);

    setThemeModeStorage(themeMode);
    setDarkMode(getIsDarkMode(themeMode));

    return () => mediaQuery.removeEventListener('change', handleThemeChange);
  }, [themeMode]);

  useEffect(() => {
    if (isDarkMode) {
      document.documentElement.classList.add('dark');
      document.documentElement.classList.remove('light');
    } else {
      document.documentElement.classList.remove('dark');
      document.documentElement.classList.add('light');
    }
  }, [isDarkMode]);

  const handleThemeChange = (newTheme: 'light' | 'dark' | 'system') => {
    setThemeMode(newTheme);
  };

  return (
    <div className={`${!horizontal ? 'px-1 py-2 space-y-2' : ''} ${className}`}>
      {!hideTitle && <div className="text-xs text-text-default px-3">Theme</div>}
      <div
        className={`${horizontal ? 'flex' : 'grid grid-cols-3'} gap-1 ${!horizontal ? 'px-3' : ''}`}
      >
        <Button
          data-testid="light-mode-button"
          onClick={() => handleThemeChange('light')}
          className={`flex items-center justify-center gap-1 p-2 rounded-md border transition-colors text-xs ${
            themeMode === 'light'
              ? 'bg-background-accent text-text-on-accent border-border-accent hover:!bg-background-accent hover:!text-text-on-accent'
              : 'border-border-default hover:!bg-background-muted text-text-muted hover:text-text-default'
          }`}
          variant="ghost"
          size="sm"
        >
          <Sun className="h-3 w-3" />
          <span>Light</span>
        </Button>

        <Button
          data-testid="dark-mode-button"
          onClick={() => handleThemeChange('dark')}
          className={`flex items-center justify-center gap-1 p-2 rounded-md border transition-colors text-xs ${
            themeMode === 'dark'
              ? 'bg-background-accent text-text-on-accent border-border-accent hover:!bg-background-accent hover:!text-text-on-accent'
              : 'border-border-default hover:!bg-background-muted text-text-muted hover:text-text-default'
          }`}
          variant="ghost"
          size="sm"
        >
          <Moon className="h-3 w-3" />
          <span>Dark</span>
        </Button>

        <Button
          data-testid="system-mode-button"
          onClick={() => handleThemeChange('system')}
          className={`flex items-center justify-center gap-1 p-2 rounded-md border transition-colors text-xs ${
            themeMode === 'system'
              ? 'bg-background-accent text-text-on-accent border-border-accent hover:!bg-background-accent hover:!text-text-on-accent'
              : 'border-border-default hover:!bg-background-muted text-text-muted hover:text-text-default'
          }`}
          variant="ghost"
          size="sm"
        >
          <Sliders className="h-3 w-3" />
          <span>System</span>
        </Button>
      </div>
    </div>
  );
};

export default ThemeSelector;
