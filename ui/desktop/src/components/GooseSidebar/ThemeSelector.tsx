import React, { useEffect, useState } from 'react';
import { Moon, Sliders, Sun } from 'lucide-react';
import { Button } from '../ui/button';
import {
  broadcastThemeChange,
  getCurrentThemeState,
  applyTheme,
  type ThemeState,
} from '../../utils/themeSync';

interface ThemeSelectorProps {
  className?: string;
  hideTitle?: boolean;
  horizontal?: boolean;
}

const ThemeSelector: React.FC<ThemeSelectorProps> = ({
  className = '',
  hideTitle = false,
  horizontal = false,
}) => {
  // Initialize state from current theme state
  const [themeState] = useState<ThemeState>(() => getCurrentThemeState());
  const [themeMode, setThemeMode] = useState<'light' | 'dark' | 'system'>(
    () => themeState.themeMode
  );
  const [isDarkMode, setDarkMode] = useState(() => themeState.isDarkMode);

  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');

    const handleSystemThemeChange = (e: { matches: boolean }) => {
      if (themeMode === 'system') {
        const newDarkMode = e.matches;
        setDarkMode(newDarkMode);

        // Broadcast the change to other windows
        const newThemeState: ThemeState = {
          themeMode: 'system',
          isDarkMode: newDarkMode,
          timestamp: Date.now(),
        };
        broadcastThemeChange(newThemeState);
      }
    };

    mediaQuery.addEventListener('change', handleSystemThemeChange);

    // Update theme based on current mode
    let newDarkMode: boolean;
    if (themeMode === 'system') {
      newDarkMode = mediaQuery.matches;
      localStorage.setItem('use_system_theme', 'true');
    } else {
      newDarkMode = themeMode === 'dark';
      localStorage.setItem('use_system_theme', 'false');
      localStorage.setItem('theme', themeMode);
    }

    setDarkMode(newDarkMode);

    // Apply theme and broadcast change
    applyTheme(newDarkMode);
    const newThemeState: ThemeState = {
      themeMode,
      isDarkMode: newDarkMode,
      timestamp: Date.now(),
    };
    broadcastThemeChange(newThemeState);

    return () => mediaQuery.removeEventListener('change', handleSystemThemeChange);
  }, [themeMode]);

  useEffect(() => {
    applyTheme(isDarkMode);
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
