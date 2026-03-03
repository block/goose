import React from 'react';
import { Moon, Sliders, Sun } from 'lucide-react';
import { Button } from '../ui/button';
import { useTheme } from '../../contexts/ThemeContext';
import { useTranslation } from 'react-i18next';

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
  const { t } = useTranslation();

  return (
    <div className={`${!horizontal ? 'px-1 py-2 space-y-2' : ''} ${className}`}>
      {!hideTitle && <div className="text-xs text-text-primary px-3">{t('settings.app.theme.title')}</div>}
      <div
        className={`${horizontal ? 'flex' : 'grid grid-cols-3'} gap-1 ${!horizontal ? 'px-3' : ''}`}
      >
        <Button
          data-testid="light-mode-button"
          onClick={() => setUserThemePreference('light')}
          className={`flex items-center justify-center gap-1 p-2 rounded-md border transition-colors text-xs ${
            userThemePreference === 'light'
              ? 'bg-background-inverse text-text-inverse border-text-inverse hover:!bg-background-inverse hover:!text-text-inverse'
              : 'border-border-primary hover:!bg-background-secondary text-text-secondary hover:text-text-primary'
          }`}
          variant="ghost"
          size="sm"
        >
          <Sun className="h-3 w-3" />
          <span>{t('theme.light')}</span>
        </Button>

        <Button
          data-testid="dark-mode-button"
          onClick={() => setUserThemePreference('dark')}
          className={`flex items-center justify-center gap-1 p-2 rounded-md border transition-colors text-xs ${
            userThemePreference === 'dark'
              ? 'bg-background-inverse text-text-inverse border-text-inverse hover:!bg-background-inverse hover:!text-text-inverse'
              : 'border-border-primary hover:!bg-background-secondary text-text-secondary hover:text-text-primary'
          }`}
          variant="ghost"
          size="sm"
        >
          <Moon className="h-3 w-3" />
          <span>{t('theme.dark')}</span>
        </Button>

        <Button
          data-testid="system-mode-button"
          onClick={() => setUserThemePreference('system')}
          className={`flex items-center justify-center gap-1 p-2 rounded-md border transition-colors text-xs ${
            userThemePreference === 'system'
              ? 'bg-background-inverse text-text-inverse border-text-inverse hover:!bg-background-inverse hover:!text-text-inverse'
              : 'border-border-primary hover:!bg-background-secondary text-text-secondary hover:text-text-primary'
          }`}
          variant="ghost"
          size="sm"
        >
          <Sliders className="h-3 w-3" />
          <span>{t('theme.system')}</span>
        </Button>
      </div>
    </div>
  );
};

export default ThemeSelector;
