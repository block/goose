/**
 * Hub Theme Selector Component
 * 
 * Allows users to select different visual themes for the Hub/Home page
 */

import React, { useState } from 'react';
import { Settings, Check } from 'lucide-react';
import { Button } from './ui/button';
import {
  HUB_THEMES,
  HUB_THEME_STORAGE_KEY,
  DEFAULT_THEME_ID,
  type HubTheme,
} from '../types/hubTheme';

interface HubThemeSelectorProps {
  currentThemeId: string;
  onThemeChange: (themeId: string) => void;
}

export const HubThemeSelector: React.FC<HubThemeSelectorProps> = ({
  currentThemeId,
  onThemeChange,
}) => {
  const [isOpen, setIsOpen] = useState(false);

  const handleThemeSelect = (themeId: string) => {
    onThemeChange(themeId);
    setIsOpen(false);
  };

  return (
    <div className="relative">
      {/* Settings Button */}
      <Button
        onClick={() => setIsOpen(!isOpen)}
        variant="ghost"
        size="sm"
        className="w-10 h-10 rounded-full bg-background-default/80 backdrop-blur-sm border border-border-default hover:bg-background-muted transition-all"
        title="Change theme"
      >
        <Settings className="w-4 h-4" />
      </Button>

      {/* Theme Selector Dropdown */}
      {isOpen && (
        <>
          {/* Backdrop */}
          <div
            className="fixed inset-0 z-40"
            onClick={() => setIsOpen(false)}
          />

          {/* Dropdown Panel */}
          <div className="absolute top-12 right-0 z-50 w-80 bg-background-default border border-border-default rounded-2xl shadow-2xl overflow-hidden">
            {/* Header */}
            <div className="px-4 py-3 border-b border-border-default bg-background-muted">
              <h3 className="text-sm font-semibold text-text-default">
                Hub Theme
              </h3>
              <p className="text-xs text-text-muted mt-0.5">
                Choose your preferred visual style
              </p>
            </div>

            {/* Theme List */}
            <div className="max-h-[400px] overflow-y-auto">
              {Object.values(HUB_THEMES).map((theme) => (
                <button
                  key={theme.id}
                  onClick={() => handleThemeSelect(theme.id)}
                  className={`w-full px-4 py-3 flex items-start gap-3 hover:bg-background-muted transition-colors text-left ${
                    currentThemeId === theme.id ? 'bg-background-subtle' : ''
                  }`}
                >
                  {/* Theme Preview */}
                  <div
                    className="flex-shrink-0 w-12 h-12 rounded-lg border border-border-default overflow-hidden relative"
                    style={{
                      background: theme.background.gradient || theme.background.color,
                    }}
                  >
                    {/* Mini ASCII preview */}
                    {theme.ascii.enabled && (
                      <div
                        className="absolute inset-0 flex items-center justify-center text-xs font-mono"
                        style={{ color: theme.ascii.color }}
                      >
                        {theme.ascii.characters.substring(0, 3)}
                      </div>
                    )}
                  </div>

                  {/* Theme Info */}
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <h4 className="text-sm font-medium text-text-default">
                        {theme.name}
                      </h4>
                      {currentThemeId === theme.id && (
                        <Check className="w-4 h-4 text-green-500 flex-shrink-0" />
                      )}
                    </div>
                    <p className="text-xs text-text-muted mt-0.5">
                      {theme.description}
                    </p>
                  </div>
                </button>
              ))}
            </div>

            {/* Footer */}
            <div className="px-4 py-2 border-t border-border-default bg-background-muted">
              <p className="text-xs text-text-muted">
                Theme applies to the home page only
              </p>
            </div>
          </div>
        </>
      )}
    </div>
  );
};

/**
 * Hook to manage hub theme state
 */
export const useHubTheme = () => {
  const [themeId, setThemeId] = useState<string>(() => {
    try {
      const stored = localStorage.getItem(HUB_THEME_STORAGE_KEY);
      return stored || DEFAULT_THEME_ID;
    } catch {
      return DEFAULT_THEME_ID;
    }
  });

  const theme = HUB_THEMES[themeId] || HUB_THEMES[DEFAULT_THEME_ID];

  const setTheme = (newThemeId: string) => {
    if (HUB_THEMES[newThemeId]) {
      setThemeId(newThemeId);
      try {
        localStorage.setItem(HUB_THEME_STORAGE_KEY, newThemeId);
      } catch (error) {
        console.error('Failed to save theme preference:', error);
      }
    }
  };

  return { theme, themeId, setTheme };
};
