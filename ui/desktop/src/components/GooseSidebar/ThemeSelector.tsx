import { useCallback, useEffect, useState } from 'react';
import { Moon, Sliders, Sun, Palette, type LucideIcon } from 'lucide-react';
import { Button } from '../ui/button';
import { Switch } from '../ui/switch';
import { CustomColorPicker } from './CustomColorPicker';
import {
  applyCustomTheme,
  resetThemeColors,
  DEFAULT_THEME_COLOR,
  isValidHexColor,
} from '../../utils/colorUtils';
import { cn } from '../../utils';

interface ThemeSelectorProps {
  className?: string;
  hideTitle?: boolean;
  horizontal?: boolean;
}

type ThemeMode = 'light' | 'dark' | 'system';

interface ThemeButtonProps {
  mode: ThemeMode;
  icon: LucideIcon;
  label: string;
  isActive: boolean;
  onClick: () => void;
}

function ThemeButton({ mode, icon: Icon, label, isActive, onClick }: ThemeButtonProps) {
  return (
    <Button
      data-testid={`${mode}-mode-button`}
      onClick={onClick}
      className={cn(
        'flex items-center justify-center gap-1 p-2 rounded-md border transition-colors text-xs',
        isActive
          ? 'bg-background-accent text-text-on-accent border-border-accent hover:!bg-background-accent hover:!text-text-on-accent'
          : 'border-border-default hover:!bg-background-muted text-text-muted hover:text-text-default'
      )}
      variant="ghost"
      size="sm"
    >
      <Icon className="h-3 w-3" />
      <span>{label}</span>
    </Button>
  );
}

const getIsDarkMode = (mode: ThemeMode): boolean => {
  if (typeof window === 'undefined') return false;
  return mode === 'system'
    ? window.matchMedia('(prefers-color-scheme: dark)').matches
    : mode === 'dark';
};

const getThemeMode = (): ThemeMode => {
  if (typeof window === 'undefined' || !window.localStorage) return 'light';
  try {
    if (localStorage.getItem('use_system_theme') === 'true') return 'system';
    const savedTheme = localStorage.getItem('theme');
    return savedTheme === 'dark' ? 'dark' : savedTheme === 'light' ? 'light' : 'light';
  } catch {
    return 'light';
  }
};

const setThemeModeStorage = (mode: ThemeMode) => {
  if (typeof window === 'undefined' || !window.localStorage) return;

  localStorage.setItem('use_system_theme', mode === 'system' ? 'true' : 'false');
  if (mode !== 'system') {
    localStorage.setItem('theme', mode);
  }

  window.electron?.broadcastThemeChange({
    mode,
    useSystemTheme: mode === 'system',
    theme: mode === 'system' ? '' : mode,
  });
};

export function ThemeSelector({
  className,
  hideTitle = false,
  horizontal = false,
}: ThemeSelectorProps) {
  const [themeMode, setThemeMode] = useState<ThemeMode>(getThemeMode);
  const [isDarkMode, setDarkMode] = useState(() => getIsDarkMode(getThemeMode()));
  const [customColor, setCustomColor] = useState(() => {
    if (typeof window === 'undefined' || !window.localStorage) return DEFAULT_THEME_COLOR;
    try {
      return localStorage.getItem('custom_theme_color') || DEFAULT_THEME_COLOR;
    } catch {
      return DEFAULT_THEME_COLOR;
    }
  });
  const [customColorEnabled, setCustomColorEnabled] = useState(() => {
    if (typeof window === 'undefined' || !window.localStorage) return false;
    try {
      return localStorage.getItem('custom_theme_enabled') === 'true';
    } catch {
      return false;
    }
  });

  useEffect(() => {
    const handleStorageChange = (e: { key: string | null; newValue: string | null }) => {
      if (e.key === 'use_system_theme' || e.key === 'theme') {
        const newThemeMode = getThemeMode();
        setThemeMode(newThemeMode);
        setDarkMode(getIsDarkMode(newThemeMode));
      }

      if (e.key === 'custom_theme_color' && e.newValue) {
        // Validate the color before setting it to state
        if (isValidHexColor(e.newValue)) {
          setCustomColor(e.newValue);
        }
      }

      if (e.key === 'custom_theme_enabled') {
        setCustomColorEnabled(e.newValue === 'true');
      }
    };

    window.addEventListener('storage', handleStorageChange);

    return () => {
      window.removeEventListener('storage', handleStorageChange);
    };
  }, []);

  useEffect(() => {
    if (typeof window === 'undefined') return;

    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');

    const handleThemeChange = (e: { matches: boolean }) => {
      if (themeMode === 'system') {
        setDarkMode(e.matches);
      }
    };

    mediaQuery.addEventListener('change', handleThemeChange);
    setThemeModeStorage(themeMode);
    setDarkMode(getIsDarkMode(themeMode));

    return () => {
      mediaQuery.removeEventListener('change', handleThemeChange);
    };
  }, [themeMode]);

  useEffect(() => {
    if (isDarkMode) {
      document.documentElement.classList.add('dark');
      document.documentElement.classList.remove('light');
    } else {
      document.documentElement.classList.remove('dark');
      document.documentElement.classList.add('light');
    }

    // Apply custom theme if enabled
    if (customColorEnabled) {
      applyCustomTheme(customColor, isDarkMode);
    } else {
      resetThemeColors();
    }
  }, [isDarkMode, customColorEnabled, customColor]);

  const handleCustomColorChange = useCallback((color: string) => {
    setCustomColor(color);
    if (typeof window !== 'undefined' && window.localStorage) {
      localStorage.setItem('custom_theme_color', color);
    }
  }, []);

  const handleCustomColorToggle = useCallback((enabled: boolean) => {
    setCustomColorEnabled(enabled);
    if (typeof window !== 'undefined' && window.localStorage) {
      localStorage.setItem('custom_theme_enabled', String(enabled));
    }
  }, []);

  return (
    <div className={cn(!horizontal && 'px-1 py-2 space-y-3', className)}>
      {!hideTitle && <div className="text-xs text-text-default px-3">Theme</div>}
      <div className={cn(horizontal ? 'flex' : 'grid grid-cols-3', 'gap-1', !horizontal && 'px-3')}>
        <ThemeButton
          mode="light"
          icon={Sun}
          label="Light"
          isActive={themeMode === 'light'}
          onClick={() => setThemeMode('light')}
        />
        <ThemeButton
          mode="dark"
          icon={Moon}
          label="Dark"
          isActive={themeMode === 'dark'}
          onClick={() => setThemeMode('dark')}
        />
        <ThemeButton
          mode="system"
          icon={Sliders}
          label="System"
          isActive={themeMode === 'system'}
          onClick={() => setThemeMode('system')}
        />
      </div>

      <div className={cn(!horizontal && 'px-3', 'pt-3 border-t border-border-default')}>
        <div className="flex items-center justify-between mb-2">
          <div className="flex items-center gap-2">
            <Palette className="h-4 w-4 text-text-muted" />
            <span className="text-xs text-text-default">Custom Accent Color</span>
          </div>
          <Switch
            checked={customColorEnabled}
            onCheckedChange={handleCustomColorToggle}
            data-testid="custom-color-toggle"
            variant="mono"
          />
        </div>

        {customColorEnabled && (
          <CustomColorPicker value={customColor} onChange={handleCustomColorChange} />
        )}
      </div>
    </div>
  );
}
