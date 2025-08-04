import React, { useState, useEffect } from 'react';
import { Button } from '../ui/button';
import { Moon, Sun, Monitor } from 'lucide-react';

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

  const [_windowOpacity, setWindowOpacity] = useState(1.0);
  const [isLoadingOpacity, setIsLoadingOpacity] = useState(true);
  const [isTransparent, setIsTransparent] = useState(false);

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

  // Load window opacity on component mount
  useEffect(() => {
    const loadWindowSettings = async () => {
      try {
        const opacity = await window.electron.getWindowOpacity();
        setWindowOpacity(opacity);
        setIsTransparent(opacity === 0.93);
      } catch (error) {
        console.error('Error loading window settings:', error);
        setWindowOpacity(1.0);
        setIsTransparent(false);
      } finally {
        setIsLoadingOpacity(false);
      }
    };

    loadWindowSettings();
  }, []);

  // handleOpacityChange function removed as it's no longer used

  const handleTransparencyToggle = async () => {
    const newTransparent = !isTransparent;
    const newOpacity = newTransparent ? 0.95 : 1.0;

    try {
      const success = await window.electron.setWindowOpacity(newOpacity);
      if (success) {
        setWindowOpacity(newOpacity);
        setIsTransparent(newTransparent);
        console.log(
          `Successfully set transparency to: ${newTransparent ? 'transparent' : 'opaque'}`
        );
      } else {
        console.error('Failed to set window transparency');
      }
    } catch (error) {
      console.error('Error setting window transparency:', error);
    }
  };

  const handleThemeChange = (newTheme: 'light' | 'dark' | 'system') => {
    setThemeMode(newTheme);
  };

  return (
    <div className={`space-y-4 ${className}`}>
      {!hideTitle && (
        <div className={`${!horizontal ? 'px-3' : ''} space-y-2`}>
          <div className="flex items-center gap-2 text-xs text-text-default">
            <span>Theme</span>
          </div>
        </div>
      )}

      <div className={`${!horizontal ? 'px-3' : ''} space-y-2`}>
        <div className="flex items-center gap-2">
          <Button
            variant={themeMode === 'light' ? 'default' : 'outline'}
            size="sm"
            onClick={() => handleThemeChange('light')}
            className="flex-1"
          >
            <Sun className="h-3 w-3 mr-1" />
            Light
          </Button>
          <Button
            variant={themeMode === 'dark' ? 'default' : 'outline'}
            size="sm"
            onClick={() => handleThemeChange('dark')}
            className="flex-1"
          >
            <Moon className="h-3 w-3 mr-1" />
            Dark
          </Button>
          <Button
            variant={themeMode === 'system' ? 'default' : 'outline'}
            size="sm"
            onClick={() => handleThemeChange('system')}
            className="flex-1"
          >
            <Monitor className="h-3 w-3 mr-1" />
            System
          </Button>
        </div>
      </div>

      {/* Window Transparency Toggle */}
      <div className={`${!horizontal ? 'px-3' : ''} space-y-2`}>
        <div className="flex items-center gap-2 text-xs text-text-default mt-5">
          <span>Window Transparency</span>
        </div>
        <div className="px-1">
          <Button
            variant={isTransparent ? 'default' : 'outline'}
            size="sm"
            onClick={handleTransparencyToggle}
            disabled={isLoadingOpacity}
            className="w-full"
          >
            {isTransparent ? 'Transparent' : 'Opaque'}
          </Button>
        </div>
      </div>
    </div>
  );
};

export default ThemeSelector;
