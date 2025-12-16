import React from 'react';
import { Moon, Sliders, Sun } from 'lucide-react';
import { Button } from '../ui/button';
import { useTheme } from '../../contexts/ThemeContext';

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
  const { userThemePreference, setUserThemePreference } = useTheme();

  return (
    <div className={`${!horizontal ? 'px-1 py-2 space-y-2' : ''} ${className}`}>
      {!hideTitle && <div className="text-xs text-text-default px-3">Theme</div>}
      <div
        className={`${horizontal ? 'flex' : 'grid grid-cols-3'} gap-1 ${!horizontal ? 'px-3' : ''}`}
      >
        <Button
          data-testid="light-mode-button"
          onClick={() => setUserThemePreference('light')}
          className={`flex items-center justify-center gap-1 p-2 rounded-md border transition-colors text-xs ${
            userThemePreference === 'light'
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
          onClick={() => setUserThemePreference('dark')}
          className={`flex items-center justify-center gap-1 p-2 rounded-md border transition-colors text-xs ${
            userThemePreference === 'dark'
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
          onClick={() => setUserThemePreference('system')}
          className={`flex items-center justify-center gap-1 p-2 rounded-md border transition-colors text-xs ${
            userThemePreference === 'system'
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
