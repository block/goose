import { useState, useCallback, useEffect } from 'react';
import { X } from 'lucide-react';
import {
  lightTokens,
  darkTokens,
  applyThemeTokens,
} from '../../../theme/theme-tokens';
import { useTheme } from '../../../contexts/ThemeContext';
import type { McpUiStyleVariableKey } from '@modelcontextprotocol/ext-apps/app-bridge';

type ThemeTokens = Record<McpUiStyleVariableKey, string>;

// Group tokens by category for organized display
const TOKEN_GROUPS: Record<string, { label: string; filter: (key: string) => boolean }> = {
  'background': { label: 'Background', filter: (k) => k.startsWith('--color-background-') },
  'text': { label: 'Text', filter: (k) => k.startsWith('--color-text-') },
  'border': { label: 'Border', filter: (k) => k.startsWith('--color-border-') },
  'ring': { label: 'Ring', filter: (k) => k.startsWith('--color-ring-') },
  'font': { label: 'Typography', filter: (k) => k.startsWith('--font-') },
  'border-radius': { label: 'Border Radius', filter: (k) => k.startsWith('--border-radius-') },
  'border-width': { label: 'Border Width', filter: (k) => k.startsWith('--border-width-') },
  'shadow': { label: 'Shadows', filter: (k) => k.startsWith('--shadow-') },
};

function isColorValue(value: string): boolean {
  return value.startsWith('#') || value.startsWith('rgb') || value.startsWith('hsl');
}

function tokenLabel(key: string): string {
  // "--color-background-primary" → "Primary"
  const parts = key.replace(/^--/, '').split('-');
  // Skip the category prefix (color-background-, color-text-, font-, etc.)
  if (parts[0] === 'color') {
    return parts.slice(2).join(' ');
  }
  if (parts[0] === 'border' && parts[1] === 'radius') {
    return parts.slice(2).join(' ');
  }
  if (parts[0] === 'border' && parts[1] === 'width') {
    return parts.slice(2).join(' ');
  }
  if (parts[0] === 'font') {
    return parts.slice(1).join(' ');
  }
  return parts.slice(1).join(' ');
}

const STORAGE_KEY = 'theme-overrides';

function loadOverrides(): { light: Partial<ThemeTokens>; dark: Partial<ThemeTokens> } {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return JSON.parse(raw);
  } catch { /* ignore */ }
  return { light: {}, dark: {} };
}

function saveOverrides(overrides: { light: Partial<ThemeTokens>; dark: Partial<ThemeTokens> }) {
  const hasOverrides = Object.keys(overrides.light).length > 0 || Object.keys(overrides.dark).length > 0;
  if (hasOverrides) {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(overrides));
  } else {
    localStorage.removeItem(STORAGE_KEY);
  }
}

interface ThemeColorEditorProps {
  onClose: () => void;
}

export function ThemeColorEditor({ onClose }: ThemeColorEditorProps) {
  const { resolvedTheme, refreshTokens } = useTheme();
  const allKeys = Object.keys(lightTokens) as McpUiStyleVariableKey[];

  const [overrides, setOverrides] = useState(() => loadOverrides());

  // Working copies that merge defaults + overrides
  const effectiveLight = { ...lightTokens, ...overrides.light };
  const effectiveDark = { ...darkTokens, ...overrides.dark };

  const activeTokens = resolvedTheme === 'dark' ? effectiveDark : effectiveLight;

  const handleChange = useCallback((key: McpUiStyleVariableKey, value: string) => {
    setOverrides((prev) => {
      const mode = resolvedTheme === 'dark' ? 'dark' : 'light';
      const defaults = mode === 'dark' ? darkTokens : lightTokens;
      const next = { ...prev };

      if (value === defaults[key]) {
        // Value matches default — remove the override
        const modeOverrides = { ...next[mode] };
        delete modeOverrides[key];
        next[mode] = modeOverrides;
      } else {
        next[mode] = { ...next[mode], [key]: value };
      }

      return next;
    });
  }, [resolvedTheme]);

  // Live preview: apply overrides as user edits
  useEffect(() => {
    const merged = resolvedTheme === 'dark'
      ? { ...darkTokens, ...overrides.dark }
      : { ...lightTokens, ...overrides.light };

    const root = document.documentElement;
    for (const [key, value] of Object.entries(merged)) {
      root.style.setProperty(key, value);
    }
  }, [overrides, resolvedTheme]);

  const handleSave = useCallback(() => {
    saveOverrides(overrides);
    refreshTokens();
    onClose();
  }, [overrides, onClose, refreshTokens]);

  const handleReset = useCallback(() => {
    const cleared = { light: {}, dark: {} };
    setOverrides(cleared);
    saveOverrides(cleared);
    applyThemeTokens(resolvedTheme);
    refreshTokens();
  }, [resolvedTheme, refreshTokens]);

  const handleCancel = useCallback(() => {
    // Revert live preview
    applyThemeTokens(resolvedTheme);
    onClose();
  }, [resolvedTheme, onClose]);

  const overrideCount = Object.keys(resolvedTheme === 'dark' ? overrides.dark : overrides.light).length;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="bg-background-primary border border-border-primary rounded-lg shadow-lg w-[700px] max-h-[80vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-border-primary">
          <div>
            <h2 className="text-lg font-semibold text-text-primary">Theme Editor</h2>
            <p className="text-sm text-text-secondary">
              Editing {resolvedTheme} mode
              {overrideCount > 0 && ` · ${overrideCount} override${overrideCount > 1 ? 's' : ''}`}
            </p>
          </div>
          <button onClick={handleCancel} className="text-text-secondary hover:text-text-primary">
            <X size={20} />
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto p-4 space-y-6">
          {Object.entries(TOKEN_GROUPS).map(([groupKey, { label, filter }]) => {
            const groupTokens = allKeys.filter(filter);
            if (groupTokens.length === 0) return null;

            return (
              <div key={groupKey}>
                <h3 className="text-sm font-semibold text-text-danger mb-3 uppercase tracking-wide">
                  {label}
                </h3>
                <div className="grid grid-cols-2 gap-x-8 gap-y-2">
                  {groupTokens.map((key) => {
                    const value = activeTokens[key];
                    const defaultValue = resolvedTheme === 'dark' ? darkTokens[key] : lightTokens[key];
                    const isOverridden = value !== defaultValue;
                    const isColor = isColorValue(value);

                    return (
                      <div key={key} className="flex items-center gap-2 py-1">
                        <span
                          className={`text-sm min-w-[140px] capitalize ${
                            isOverridden ? 'text-text-danger font-medium' : 'text-text-secondary'
                          }`}
                        >
                          {tokenLabel(key)}
                          {isOverridden && ' •'}
                        </span>

                        {isColor ? (
                          <div className="flex items-center gap-1">
                            <input
                              type="color"
                              value={value}
                              onChange={(e) => handleChange(key, e.target.value)}
                              className="w-8 h-8 rounded border border-border-primary cursor-pointer"
                            />
                            <input
                              type="text"
                              value={value}
                              onChange={(e) => handleChange(key, e.target.value)}
                              className="w-[80px] text-xs font-mono bg-background-secondary text-text-primary border border-border-primary rounded px-1.5 py-0.5"
                            />
                          </div>
                        ) : (
                          <input
                            type="text"
                            value={value}
                            onChange={(e) => handleChange(key, e.target.value)}
                            className="flex-1 text-xs font-mono bg-background-secondary text-text-primary border border-border-primary rounded px-1.5 py-0.5"
                          />
                        )}
                      </div>
                    );
                  })}
                </div>
              </div>
            );
          })}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between p-4 border-t border-border-primary">
          <button
            onClick={handleReset}
            className="text-sm text-text-secondary hover:text-text-primary underline"
          >
            Reset to Default
          </button>
          <div className="flex gap-2">
            <button
              onClick={handleCancel}
              className="px-4 py-2 text-sm border border-border-primary rounded-md text-text-primary hover:bg-background-secondary"
            >
              Cancel
            </button>
            <button
              onClick={handleSave}
              className="px-4 py-2 text-sm bg-text-primary text-background-primary rounded-md hover:opacity-90"
            >
              Save Changes
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
