import React, { useEffect, useState } from 'react';
import { Moon, Sliders, Sun } from 'lucide-react';
import { Button } from '../ui/button';

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
  const [themeMode, setThemeMode] = useState<'light' | 'dark' | 'system'>(() => {
    const savedUseSystemTheme = localStorage.getItem('use_system_theme') === 'true';
    if (savedUseSystemTheme) {
      return 'system';
    }
    const savedTheme = localStorage.getItem('theme');
    return savedTheme === 'dark' ? 'dark' : 'light';
  });

  const [isDarkMode, setDarkMode] = useState(() => {
    // First check localStorage to determine the intended theme
    const savedUseSystemTheme = localStorage.getItem('use_system_theme') === 'true';
    const savedTheme = localStorage.getItem('theme');

    if (savedUseSystemTheme) {
      // Use system preference
      const systemPrefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      return systemPrefersDark;
    } else if (savedTheme) {
      // Use saved theme preference
      return savedTheme === 'dark';
    } else {
      // Fallback: check current DOM state to maintain consistency
      return document.documentElement.classList.contains('dark');
    }
  });

  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');

    const handleThemeChange = (e: { matches: boolean }) => {
      if (themeMode === 'system') {
        setDarkMode(e.matches);
      }
    };

    mediaQuery.addEventListener('change', handleThemeChange);

    if (themeMode === 'system') {
      setDarkMode(mediaQuery.matches);
      localStorage.setItem('use_system_theme', 'true');
    } else {
      setDarkMode(themeMode === 'dark');
      localStorage.setItem('use_system_theme', 'false');
      localStorage.setItem('theme', themeMode);
    }

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
