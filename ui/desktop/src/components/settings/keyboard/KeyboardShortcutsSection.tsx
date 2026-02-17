import { useState, useEffect, useCallback } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';
import { Button } from '../../ui/button';
import { Switch } from '../../ui/switch';
import { ShortcutRecorder } from './ShortcutRecorder';
import { KeyboardShortcuts, defaultKeyboardShortcuts } from '../../../utils/settings';
import { trackSettingToggled } from '../../../utils/analytics';

interface ShortcutConfig {
  key: keyof KeyboardShortcuts;
  label: string;
  description: string;
  category: 'global' | 'application' | 'search' | 'window';
}

const shortcutConfigs: ShortcutConfig[] = [
  {
    key: 'focusWindow',
    label: 'Focus Goose Window',
    description: 'Bring Goose window to front from anywhere',
    category: 'global',
  },
  {
    key: 'quickLauncher',
    label: 'Quick Launcher',
    description: 'Open the quick launcher overlay',
    category: 'global',
  },
  {
    key: 'newChat',
    label: 'New Chat',
    description: 'Create a new chat in the current window',
    category: 'application',
  },
  {
    key: 'newChatWindow',
    label: 'New Chat Window',
    description: 'Open a new Goose window',
    category: 'application',
  },
  {
    key: 'openDirectory',
    label: 'Open Directory',
    description: 'Open directory selection dialog',
    category: 'application',
  },
  {
    key: 'settings',
    label: 'Settings',
    description: 'Open settings panel',
    category: 'application',
  },
  {
    key: 'find',
    label: 'Find',
    description: 'Open search in conversation',
    category: 'search',
  },
  {
    key: 'findNext',
    label: 'Find Next',
    description: 'Jump to next search result',
    category: 'search',
  },
  {
    key: 'findPrevious',
    label: 'Find Previous',
    description: 'Jump to previous search result',
    category: 'search',
  },
  {
    key: 'alwaysOnTop',
    label: 'Always on Top',
    description: 'Toggle window always on top',
    category: 'window',
  },
];

const needsRestart = new Set<keyof KeyboardShortcuts>([
  'newChat',
  'newChatWindow',
  'openDirectory',
  'settings',
  'find',
  'findNext',
  'findPrevious',
  'alwaysOnTop',
]);

export const getShortcutLabel = (key: string): string => {
  const config = shortcutConfigs.find((c) => c.key === key);
  return config?.label || key;
};

export const formatShortcut = (shortcut: string): string => {
  const isMac = window.electron.platform === 'darwin';
  return shortcut
    .replace('CommandOrControl', isMac ? '⌘' : 'Ctrl')
    .replace('Command', '⌘')
    .replace('Control', 'Ctrl')
    .replace('Alt', isMac ? '⌥' : 'Alt')
    .replace('Shift', isMac ? '⇧' : 'Shift');
};

const categoryLabels = {
  global: 'Global Shortcuts',
  application: 'Application Shortcuts',
  search: 'Search Shortcuts',
  window: 'Window Shortcuts',
};

const categoryDescriptions = {
  global: 'These shortcuts work system-wide, even when Goose is not focused',
  application: 'These shortcuts work when Goose is the active application',
  search: 'These shortcuts work when searching in a conversation',
  window: 'These shortcuts control window behavior',
};

export default function KeyboardShortcutsSection() {
  const [shortcuts, setShortcuts] = useState<KeyboardShortcuts | null>(null);
  const [editingKey, setEditingKey] = useState<keyof KeyboardShortcuts | null>(null);
  const [showRestartNotice, setShowRestartNotice] = useState(false);

  const loadShortcuts = useCallback(async () => {
    const keyboardShortcuts = await window.electron.getSetting('keyboardShortcuts');
    setShortcuts(keyboardShortcuts || defaultKeyboardShortcuts);
  }, []);

  useEffect(() => {
    loadShortcuts();
  }, [loadShortcuts]);

  const handleToggle = async (key: keyof KeyboardShortcuts, enabled: boolean) => {
    if (!shortcuts) return;

    const defaultValue = defaultKeyboardShortcuts[key];
    const newShortcuts = { ...shortcuts };

    if (enabled) {
      const conflictingKey = Object.entries(shortcuts).find(
        ([k, value]) => k !== key && value === defaultValue
      )?.[0];

      if (conflictingKey) {
        const confirmed = await window.electron.showMessageBox({
          type: 'warning',
          title: 'Shortcut Conflict',
          message: `The shortcut ${formatShortcut(defaultValue)} is already assigned to "${getShortcutLabel(conflictingKey)}".`,
          detail: `Enabling this will remove the shortcut from "${getShortcutLabel(conflictingKey)}" and assign it to "${getShortcutLabel(key)}". Do you want to continue?`,
          buttons: ['Reassign Shortcut', 'Cancel'],
          defaultId: 1,
        });

        if (confirmed.response !== 0) {
          return;
        }

        newShortcuts[conflictingKey as keyof KeyboardShortcuts] = null;
      }

      newShortcuts[key] = defaultValue;
    } else {
      newShortcuts[key] = null;
    }

    await window.electron.setSetting('keyboardShortcuts', newShortcuts);
    setShortcuts(newShortcuts);
    trackSettingToggled(`shortcut_${key}`, enabled);
    if (needsRestart.has(key)) {
      setShowRestartNotice(true);
    }
  };

  const handleEdit = (key: keyof KeyboardShortcuts) => {
    setEditingKey(key);
  };

  const handleSave = async (shortcut: string) => {
    if (!shortcuts || !editingKey) return;

    const conflictingKey = Object.entries(shortcuts).find(
      ([key, value]) => key !== editingKey && value === shortcut
    )?.[0];

    if (conflictingKey) {
      const confirmed = await window.electron.showMessageBox({
        type: 'warning',
        title: 'Shortcut Conflict',
        message: `The shortcut ${formatShortcut(shortcut)} is already assigned to "${getShortcutLabel(conflictingKey)}".`,
        detail: `Saving this will remove the shortcut from "${getShortcutLabel(conflictingKey)}" and assign it to "${getShortcutLabel(editingKey)}". Do you want to continue?`,
        buttons: ['Reassign Shortcut', 'Cancel'],
        defaultId: 1,
      });

      if (confirmed.response !== 0) {
        return;
      }
    }

    const newShortcuts = { ...shortcuts };

    if (conflictingKey) {
      newShortcuts[conflictingKey as keyof KeyboardShortcuts] = null;
    }

    newShortcuts[editingKey] = shortcut || null;

    await window.electron.setSetting('keyboardShortcuts', newShortcuts);
    setShortcuts(newShortcuts);
    setEditingKey(null);
    if (needsRestart.has(editingKey)) {
      setShowRestartNotice(true);
    }
  };

  const handleCancel = () => {
    setEditingKey(null);
  };

  const handleResetToDefaults = async () => {
    const confirmed = await window.electron.showMessageBox({
      type: 'question',
      title: 'Reset Keyboard Shortcuts',
      message: 'Reset all keyboard shortcuts to their default values?',
      detail: 'This will restore all shortcuts to their original configuration.',
      buttons: ['Reset to Defaults', 'Cancel'],
      defaultId: 1,
    });

    if (confirmed.response === 0) {
      await window.electron.setSetting('keyboardShortcuts', { ...defaultKeyboardShortcuts });
      setShortcuts({ ...defaultKeyboardShortcuts });
      setShowRestartNotice(true);
      trackSettingToggled('shortcuts_reset', true);
    }
  };

  const groupedShortcuts = shortcutConfigs.reduce(
    (acc, config) => {
      if (!acc[config.category]) {
        acc[config.category] = [];
      }
      acc[config.category].push(config);
      return acc;
    },
    {} as Record<string, ShortcutConfig[]>
  );

  if (!shortcuts) {
    return <div>Loading...</div>;
  }

  return (
    <div className="space-y-4 pr-4 pb-8 mt-1">
      {showRestartNotice && (
        <Card className="rounded-lg border-yellow-600/50 bg-yellow-600/10">
          <CardContent className="pt-4 px-4 pb-4">
            <div className="flex items-start justify-between gap-4">
              <div className="flex-1">
                <h3 className="text-text-primary text-sm font-medium mb-1">Restart Required</h3>
                <p className="text-xs text-text-secondary">
                  Changes to application shortcuts (like New Chat, Settings, etc.) require
                  restarting Goose to take effect. Global shortcuts (Focus Window, Quick Launcher)
                  work immediately.
                </p>
              </div>
              <Button
                variant="secondary"
                size="sm"
                onClick={() => setShowRestartNotice(false)}
                className="text-xs shrink-0"
              >
                Dismiss
              </Button>
            </div>
          </CardContent>
        </Card>
      )}
      {Object.entries(groupedShortcuts).map(([category, configs]) => (
        <Card key={category} className="rounded-lg">
          <CardHeader className="pb-0">
            <CardTitle>{categoryLabels[category as keyof typeof categoryLabels]}</CardTitle>
            <CardDescription>
              {categoryDescriptions[category as keyof typeof categoryDescriptions]}
            </CardDescription>
          </CardHeader>
          <CardContent className="pt-4 space-y-4 px-4">
            {configs.map((config) => {
              const shortcut = shortcuts[config.key];
              const isEditing = editingKey === config.key;

              return (
                <div key={config.key} className="flex items-center justify-between">
                  <div className="flex-1">
                    <h3 className="text-text-primary text-xs">{config.label}</h3>
                    <p className="text-xs text-text-secondary max-w-md mt-[2px]">
                      {config.description}
                    </p>
                  </div>
                  <div className="flex items-center gap-2">
                    {!isEditing ? (
                      <>
                        {shortcut ? (
                          <span className="text-xs font-mono px-2 py-1 bg-background-secondary rounded min-w-[120px] text-center">
                            {formatShortcut(shortcut)}
                          </span>
                        ) : (
                          <span className="text-xs text-text-secondary min-w-[120px] text-center">
                            Disabled
                          </span>
                        )}
                        <Button
                          variant="secondary"
                          size="sm"
                          onClick={() => handleEdit(config.key)}
                          className="text-xs"
                        >
                          Change
                        </Button>
                        <Switch
                          checked={shortcut !== null}
                          onCheckedChange={(checked) => handleToggle(config.key, checked)}
                          variant="mono"
                        />
                      </>
                    ) : (
                      <ShortcutRecorder
                        value={shortcut || ''}
                        onSave={handleSave}
                        onCancel={handleCancel}
                        allShortcuts={shortcuts}
                        currentKey={config.key}
                      />
                    )}
                  </div>
                </div>
              );
            })}
          </CardContent>
        </Card>
      ))}

      <Card className="rounded-lg">
        <CardContent className="pt-4 px-4 pb-4">
          <div className="flex items-center justify-between">
            <div>
              <h3 className="text-text-primary text-sm font-medium">Reset to Defaults</h3>
              <p className="text-xs text-text-secondary max-w-md mt-[2px]">
                Restore all keyboard shortcuts to their original configuration
              </p>
            </div>
            <Button
              variant="secondary"
              size="sm"
              onClick={handleResetToDefaults}
              className="text-xs"
            >
              Reset All Shortcuts
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
