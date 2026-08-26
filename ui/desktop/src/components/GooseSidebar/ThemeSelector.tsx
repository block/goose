import React from 'react';
import { Moon, Sliders, Sparkles, Sun } from 'lucide-react';
import { Button } from '../ui/button';
import { useTheme } from '../../contexts/ThemeContext';
import { defineMessages, useIntl } from '../../i18n';
import { CATPPUCCIN_ACCENTS, CATPPUCCIN_THEME_IDS, isCatppuccinThemeId } from '../../theme/types';
import { getCatppuccinPreview } from '../../theme/catppuccin';
import type { CatppuccinAccent, CatppuccinThemeId } from '../../theme/types';

const i18n = defineMessages({
  theme: {
    id: 'themeSelector.theme',
    defaultMessage: 'Theme',
  },
  light: {
    id: 'themeSelector.light',
    defaultMessage: 'Light',
  },
  dark: {
    id: 'themeSelector.dark',
    defaultMessage: 'Dark',
  },
  aura: {
    id: 'themeSelector.aura',
    defaultMessage: 'Aura',
  },
  system: {
    id: 'themeSelector.system',
    defaultMessage: 'System',
  },
  builtinGroup: {
    id: 'themeSelector.builtinGroup',
    defaultMessage: 'Built-in',
  },
  catppuccinGroup: {
    id: 'themeSelector.catppuccinGroup',
    defaultMessage: 'Catppuccin',
  },
  accent: {
    id: 'themeSelector.accent',
    defaultMessage: 'Accent',
  },
  latte: {
    id: 'themeSelector.catppuccinLatte',
    defaultMessage: 'Latte',
  },
  frappe: {
    id: 'themeSelector.catppuccinFrappe',
    defaultMessage: 'Frappé',
  },
  macchiato: {
    id: 'themeSelector.catppuccinMacchiato',
    defaultMessage: 'Macchiato',
  },
  mocha: {
    id: 'themeSelector.catppuccinMocha',
    defaultMessage: 'Mocha',
  },
  rosewater: { id: 'themeSelector.accentRosewater', defaultMessage: 'Rosewater' },
  flamingo: { id: 'themeSelector.accentFlamingo', defaultMessage: 'Flamingo' },
  pink: { id: 'themeSelector.accentPink', defaultMessage: 'Pink' },
  mauve: { id: 'themeSelector.accentMauve', defaultMessage: 'Mauve' },
  red: { id: 'themeSelector.accentRed', defaultMessage: 'Red' },
  maroon: { id: 'themeSelector.accentMaroon', defaultMessage: 'Maroon' },
  peach: { id: 'themeSelector.accentPeach', defaultMessage: 'Peach' },
  yellow: { id: 'themeSelector.accentYellow', defaultMessage: 'Yellow' },
  green: { id: 'themeSelector.accentGreen', defaultMessage: 'Green' },
  teal: { id: 'themeSelector.accentTeal', defaultMessage: 'Teal' },
  sky: { id: 'themeSelector.accentSky', defaultMessage: 'Sky' },
  sapphire: { id: 'themeSelector.accentSapphire', defaultMessage: 'Sapphire' },
  blue: { id: 'themeSelector.accentBlue', defaultMessage: 'Blue' },
  lavender: { id: 'themeSelector.accentLavender', defaultMessage: 'Lavender' },
});

const flavorMessage: Record<CatppuccinThemeId, keyof typeof i18n> = {
  'catppuccin-latte': 'latte',
  'catppuccin-frappe': 'frappe',
  'catppuccin-macchiato': 'macchiato',
  'catppuccin-mocha': 'mocha',
};

interface ThemeSelectorProps {
  className?: string;
  hideTitle?: boolean;
  horizontal?: boolean;
}

function builtinButtonClass(selected: boolean): string {
  return `flex items-center justify-center gap-1 p-2 rounded-md border transition-colors text-xs ${
    selected
      ? 'bg-background-inverse text-text-inverse border-text-inverse hover:!bg-background-inverse hover:!text-text-inverse'
      : 'border-border-primary hover:!bg-background-secondary text-text-secondary hover:text-text-primary'
  }`;
}

const ThemeSelector: React.FC<ThemeSelectorProps> = ({
  className = '',
  hideTitle = false,
  horizontal = false,
}) => {
  const intl = useIntl();
  const {
    userThemePreference,
    setUserThemePreference,
    catppuccinAccent,
    setCatppuccinAccent,
  } = useTheme();

  const catppuccinSelected = isCatppuccinThemeId(userThemePreference);

  return (
    <div className={`space-y-4 ${className}`}>
      {!hideTitle && (
        <div className="text-xs text-text-primary">{intl.formatMessage(i18n.theme)}</div>
      )}

      <div className="space-y-2">
        <div className="text-[11px] uppercase tracking-wide text-text-tertiary">
          {intl.formatMessage(i18n.builtinGroup)}
        </div>
        <div
          className={`${horizontal ? 'flex flex-wrap' : 'grid grid-cols-4'} gap-1`}
        >
          <Button
            data-testid="light-mode-button"
            onClick={() => setUserThemePreference('light')}
            className={builtinButtonClass(userThemePreference === 'light')}
            variant="ghost"
            size="sm"
          >
            <Sun className="h-3 w-3" />
            <span>{intl.formatMessage(i18n.light)}</span>
          </Button>

          <Button
            data-testid="dark-mode-button"
            onClick={() => setUserThemePreference('dark')}
            className={builtinButtonClass(userThemePreference === 'dark')}
            variant="ghost"
            size="sm"
          >
            <Moon className="h-3 w-3" />
            <span>{intl.formatMessage(i18n.dark)}</span>
          </Button>

          <Button
            data-testid="aura-mode-button"
            onClick={() => setUserThemePreference('aura')}
            className={builtinButtonClass(userThemePreference === 'aura')}
            variant="ghost"
            size="sm"
          >
            <Sparkles className="h-3 w-3" />
            <span>{intl.formatMessage(i18n.aura)}</span>
          </Button>

          <Button
            data-testid="system-mode-button"
            onClick={() => setUserThemePreference('system')}
            className={builtinButtonClass(userThemePreference === 'system')}
            variant="ghost"
            size="sm"
          >
            <Sliders className="h-3 w-3" />
            <span>{intl.formatMessage(i18n.system)}</span>
          </Button>
        </div>
      </div>

      <div className="space-y-2">
        <div className="text-[11px] uppercase tracking-wide text-text-tertiary">
          {intl.formatMessage(i18n.catppuccinGroup)}
        </div>
        <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
          {CATPPUCCIN_THEME_IDS.map((flavorId) => (
            <CatppuccinFlavorCard
              key={flavorId}
              flavorId={flavorId}
              selected={userThemePreference === flavorId}
              accent={catppuccinAccent}
              label={intl.formatMessage(i18n[flavorMessage[flavorId]])}
              onSelect={() => setUserThemePreference(flavorId)}
            />
          ))}
        </div>
      </div>

      {catppuccinSelected && (
        <div className="space-y-2">
          <div className="text-[11px] uppercase tracking-wide text-text-tertiary">
            {intl.formatMessage(i18n.accent)}
          </div>
          <div className="flex flex-wrap gap-1.5">
            {CATPPUCCIN_ACCENTS.map((accent) => {
              const preview = getCatppuccinPreview(userThemePreference, accent);
              const selected = catppuccinAccent === accent;
              return (
                <button
                  key={accent}
                  type="button"
                  data-testid={`catppuccin-accent-${accent}`}
                  aria-label={intl.formatMessage(i18n[accent])}
                  title={intl.formatMessage(i18n[accent])}
                  onClick={() => setCatppuccinAccent(accent)}
                  className={`h-5 w-5 rounded-full border transition-transform ${
                    selected
                      ? 'scale-110 ring-2 ring-offset-2 ring-text-primary ring-offset-background-primary'
                      : 'border-border-primary hover:scale-105'
                  }`}
                  style={{ backgroundColor: preview.accent }}
                />
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
};

interface CatppuccinFlavorCardProps {
  flavorId: CatppuccinThemeId;
  selected: boolean;
  accent: CatppuccinAccent;
  label: string;
  onSelect: () => void;
}

const CatppuccinFlavorCard: React.FC<CatppuccinFlavorCardProps> = ({
  flavorId,
  selected,
  accent,
  label,
  onSelect,
}) => {
  const preview = getCatppuccinPreview(flavorId, accent);

  return (
    <button
      type="button"
      data-testid={`${flavorId}-theme-button`}
      aria-label={label}
      aria-pressed={selected}
      onClick={onSelect}
      className={`overflow-hidden rounded-md border text-left transition-colors ${
        selected
          ? 'border-text-primary ring-1 ring-text-primary'
          : 'border-border-primary hover:border-border-secondary'
      }`}
    >
      <div className="flex h-14" style={{ backgroundColor: preview.background }}>
        <div className="w-3 shrink-0" style={{ backgroundColor: preview.sidebar }} />
        <div className="flex flex-1 flex-col justify-between p-1.5">
          <div className="flex items-center gap-1">
            <span
              className="h-1.5 w-8 rounded-sm"
              style={{ backgroundColor: preview.text, opacity: 0.85 }}
            />
            <span
              className="h-1.5 w-3 rounded-sm"
              style={{ backgroundColor: preview.accent }}
            />
          </div>
          <div
            className="h-5 rounded-sm"
            style={{ backgroundColor: preview.surface }}
          >
            <div className="flex h-full items-center gap-1 px-1">
              <span
                className="h-1 w-6 rounded-sm"
                style={{ backgroundColor: preview.muted }}
              />
              <span
                className="ml-auto h-2 w-2 rounded-full"
                style={{ backgroundColor: preview.accent }}
              />
            </div>
          </div>
        </div>
      </div>
      <div className="bg-background-primary px-2 py-1 text-[11px] text-text-secondary">
        {label}
      </div>
    </button>
  );
};

export default ThemeSelector;
