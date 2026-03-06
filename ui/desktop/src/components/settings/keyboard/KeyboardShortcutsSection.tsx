import { useState, useEffect, useCallback } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';
import { Button } from '../../ui/button';
import { Switch } from '../../ui/switch';
import { ShortcutRecorder } from './ShortcutRecorder';
import { KeyboardShortcuts, defaultKeyboardShortcuts } from '../../../utils/settings';
import { trackSettingToggled } from '../../../utils/analytics';
import { useLocalization } from '../../../contexts/LocalizationContext';
import type { TranslationKey } from '../../../i18n';

interface ShortcutConfig {
  key: keyof KeyboardShortcuts;
  label: string;
  description: string;
  category: 'global' | 'application' | 'search' | 'window';
}

const createShortcutConfigs = (
  t: (key: TranslationKey, params?: Record<string, string | number>) => string
): ShortcutConfig[] => [
  {
    key: 'focusWindow',
    label: t('keyboardShortcuts.shortcuts.focusWindow.label'),
    description: t('keyboardShortcuts.shortcuts.focusWindow.description'),
    category: 'global',
  },
  {
    key: 'quickLauncher',
    label: t('keyboardShortcuts.shortcuts.quickLauncher.label'),
    description: t('keyboardShortcuts.shortcuts.quickLauncher.description'),
    category: 'global',
  },
  {
    key: 'newChat',
    label: t('keyboardShortcuts.shortcuts.newChat.label'),
    description: t('keyboardShortcuts.shortcuts.newChat.description'),
    category: 'application',
  },
  {
    key: 'newChatWindow',
    label: t('keyboardShortcuts.shortcuts.newChatWindow.label'),
    description: t('keyboardShortcuts.shortcuts.newChatWindow.description'),
    category: 'application',
  },
  {
    key: 'openDirectory',
    label: t('keyboardShortcuts.shortcuts.openDirectory.label'),
    description: t('keyboardShortcuts.shortcuts.openDirectory.description'),
    category: 'application',
  },
  {
    key: 'settings',
    label: t('keyboardShortcuts.shortcuts.settings.label'),
    description: t('keyboardShortcuts.shortcuts.settings.description'),
    category: 'application',
  },
  {
    key: 'find',
    label: t('keyboardShortcuts.shortcuts.find.label'),
    description: t('keyboardShortcuts.shortcuts.find.description'),
    category: 'search',
  },
  {
    key: 'findNext',
    label: t('keyboardShortcuts.shortcuts.findNext.label'),
    description: t('keyboardShortcuts.shortcuts.findNext.description'),
    category: 'search',
  },
  {
    key: 'findPrevious',
    label: t('keyboardShortcuts.shortcuts.findPrevious.label'),
    description: t('keyboardShortcuts.shortcuts.findPrevious.description'),
    category: 'search',
  },
  {
    key: 'alwaysOnTop',
    label: t('keyboardShortcuts.shortcuts.alwaysOnTop.label'),
    description: t('keyboardShortcuts.shortcuts.alwaysOnTop.description'),
    category: 'window',
  },
  {
    key: 'toggleNavigation',
    label: t('keyboardShortcuts.shortcuts.toggleNavigation.label'),
    description: t('keyboardShortcuts.shortcuts.toggleNavigation.description'),
    category: 'application',
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

export const getShortcutLabel = (key: string, shortcutConfigs: ShortcutConfig[]): string => {
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

export default function KeyboardShortcutsSection() {
  const { t } = useLocalization();
  const shortcutConfigs = createShortcutConfigs(t);
  const categoryLabels: Record<ShortcutConfig['category'], string> = {
    global: t('keyboardShortcuts.categories.global'),
    application: t('keyboardShortcuts.categories.application'),
    search: t('keyboardShortcuts.categories.search'),
    window: t('keyboardShortcuts.categories.window'),
  };
  const categoryDescriptions: Record<ShortcutConfig['category'], string> = {
    global: t('keyboardShortcuts.categoryDescriptions.global'),
    application: t('keyboardShortcuts.categoryDescriptions.application'),
    search: t('keyboardShortcuts.categoryDescriptions.search'),
    window: t('keyboardShortcuts.categoryDescriptions.window'),
  };
  const [shortcuts, setShortcuts] = useState<KeyboardShortcuts | null>(null);
  const [editingKey, setEditingKey] = useState<keyof KeyboardShortcuts | null>(null);
  const [showRestartNotice, setShowRestartNotice] = useState(false);

  const loadShortcuts = useCallback(async () => {
    const keyboardShortcuts = await window.electron.getSetting('keyboardShortcuts');
    setShortcuts({ ...defaultKeyboardShortcuts, ...keyboardShortcuts });
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
          title: t('keyboardShortcuts.dialogs.conflictTitle'),
          message: t('keyboardShortcuts.dialogs.conflictMessage', {
            shortcut: formatShortcut(defaultValue),
            label: getShortcutLabel(conflictingKey, shortcutConfigs),
          }),
          detail: t('keyboardShortcuts.dialogs.enableConflictDetail', {
            existing: getShortcutLabel(conflictingKey, shortcutConfigs),
            current: getShortcutLabel(key, shortcutConfigs),
          }),
          buttons: [t('keyboardShortcuts.dialogs.reassign'), t('common.actions.cancel')],
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
        title: t('keyboardShortcuts.dialogs.conflictTitle'),
        message: t('keyboardShortcuts.dialogs.conflictMessage', {
          shortcut: formatShortcut(shortcut),
          label: getShortcutLabel(conflictingKey, shortcutConfigs),
        }),
        detail: t('keyboardShortcuts.dialogs.saveConflictDetail', {
          existing: getShortcutLabel(conflictingKey, shortcutConfigs),
          current: getShortcutLabel(editingKey, shortcutConfigs),
        }),
        buttons: [t('keyboardShortcuts.dialogs.reassign'), t('common.actions.cancel')],
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
      title: t('keyboardShortcuts.dialogs.resetTitle'),
      message: t('keyboardShortcuts.dialogs.resetMessage'),
      detail: t('keyboardShortcuts.dialogs.resetDetail'),
      buttons: [t('keyboardShortcuts.dialogs.resetConfirm'), t('common.actions.cancel')],
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
    return <div>{t('keyboardShortcuts.loading')}</div>;
  }

  return (
    <div className="space-y-4 pr-4 pb-8 mt-1">
      {showRestartNotice && (
        <Card className="rounded-lg border-yellow-600/50 bg-yellow-600/10">
          <CardContent className="pt-4 px-4 pb-4">
            <div className="flex items-start justify-between gap-4">
              <div className="flex-1">
                <h3 className="text-text-primary text-sm font-medium mb-1">
                  {t('keyboardShortcuts.restartRequiredTitle')}
                </h3>
                <p className="text-xs text-text-secondary">
                  {t('keyboardShortcuts.restartRequiredDescription')}
                </p>
              </div>
              <Button
                variant="secondary"
                size="sm"
                onClick={() => setShowRestartNotice(false)}
                className="text-xs shrink-0"
              >
                {t('keyboardShortcuts.dismiss')}
              </Button>
            </div>
          </CardContent>
        </Card>
      )}
      {Object.entries(groupedShortcuts).map(([category, configs]) => (
        <Card key={category} className="rounded-lg">
          <CardHeader className="pb-0">
            <CardTitle>{categoryLabels[category as ShortcutConfig['category']]}</CardTitle>
            <CardDescription>
              {categoryDescriptions[category as ShortcutConfig['category']]}
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
                            {t('keyboardShortcuts.disabled')}
                          </span>
                        )}
                        <Button
                          variant="secondary"
                          size="sm"
                          onClick={() => handleEdit(config.key)}
                          className="text-xs"
                        >
                          {t('keyboardShortcuts.change')}
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
                        resolveShortcutLabel={(key) => getShortcutLabel(key, shortcutConfigs)}
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
              <h3 className="text-text-primary text-sm font-medium">
                {t('keyboardShortcuts.resetTitle')}
              </h3>
              <p className="text-xs text-text-secondary max-w-md mt-[2px]">
                {t('keyboardShortcuts.resetDescription')}
              </p>
            </div>
            <Button
              variant="secondary"
              size="sm"
              onClick={handleResetToDefaults}
              className="text-xs"
            >
              {t('keyboardShortcuts.resetAll')}
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
