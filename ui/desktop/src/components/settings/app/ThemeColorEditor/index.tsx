/**
 * ThemeColorEditor Component
 *
 * Theme customization with preset gallery and custom color picking.
 * Fully client-side — no server round-trips. Presets and custom themes
 * are stored in localStorage and applied via style.setProperty().
 */

import { useState, useEffect } from 'react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from '../../../ui/dialog';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../../../ui/tabs';
import { Button } from '../../../ui/button';
import {
  ThemeColorEditorProps,
  ThemeColors,
  ColorMode,
  COLOR_VARIABLES,
} from './types';
import { toast } from 'react-toastify';
import { SimpleColorPicker } from './ColorPicker/SimpleColorPicker';
import { PresetGallery } from './ThemeSelector/PresetGallery';
import { ColorPreview } from './Preview/ColorPreview';
import { RotateCcw, Save, Palette, Paintbrush, Pipette } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipTrigger } from '../../../ui/Tooltip';
import { useTheme } from '../../../../contexts/ThemeContext';
import { lightTokens, darkTokens } from '../../../../theme/theme-tokens';
import {
  saveCustomTheme as persistCustomTheme,
  getThemePreset,
} from '../../../../themes/presets';
import type { ThemePreset } from '../../../../themes/presets/types';

/** Strip the `--` prefix so keys match the preset format (`color-background-primary`). */
function stripPrefix(key: string): string {
  return key.startsWith('--') ? key.slice(2) : key;
}

/** Build a ThemeColors object from the current token defaults, optionally overlaying a preset. */
function buildColorsFromTokens(preset?: ThemePreset): ThemeColors {
  const light: Record<string, string> = {};
  const dark: Record<string, string> = {};

  for (const [key, value] of Object.entries(lightTokens)) {
    if (key.startsWith('--color-')) {
      light[stripPrefix(key)] = value;
    }
  }
  for (const [key, value] of Object.entries(darkTokens)) {
    if (key.startsWith('--color-')) {
      dark[stripPrefix(key)] = value;
    }
  }

  if (preset) {
    Object.assign(light, preset.colors.light);
    Object.assign(dark, preset.colors.dark);
  }

  return { light, dark };
}

export function ThemeColorEditor({ onClose }: ThemeColorEditorProps) {
  const { resolvedTheme, applyPreset, activePresetId } = useTheme();
  const [saving, setSaving] = useState(false);
  const [themeColors, setThemeColors] = useState<ThemeColors>(() => {
    const preset = activePresetId ? getThemePreset(activePresetId) : undefined;
    return buildColorsFromTokens(preset);
  });
  const [activeTab, setActiveTab] = useState<'presets' | 'customize'>('presets');
  const [selectedVariable, setSelectedVariable] = useState<string | null>(null);
  const [themeName, setThemeName] = useState('My Custom Theme');
  const [themeDescription, setThemeDescription] = useState('');
  const [editingThemeId, setEditingThemeId] = useState<string | null>(null);

  const activeMode: ColorMode = resolvedTheme;

  // Sync colors when active preset changes externally
  useEffect(() => {
    const preset = activePresetId ? getThemePreset(activePresetId) : undefined;
    setThemeColors(buildColorsFromTokens(preset));
  }, [activePresetId]);

  const handleColorChange = (variableName: string, color: string) => {
    setThemeColors((prev) => ({
      ...prev,
      [activeMode]: {
        ...prev[activeMode],
        [variableName]: color,
      },
    }));
  };

  const handleSave = () => {
    if (!themeName.trim()) {
      toast.error('Please enter a theme name');
      return;
    }

    setSaving(true);

    const themeId =
      editingThemeId ||
      `custom-${themeName.toLowerCase().replace(/[^a-z0-9]+/g, '-')}-${Date.now()}`;

    // Persist to localStorage
    persistCustomTheme({
      id: themeId,
      name: themeName.trim(),
      author: 'You',
      description: themeDescription.trim() || 'Custom theme',
      tags: ['custom'],
      colors: {
        light: themeColors.light,
        dark: themeColors.dark,
      },
      version: '1.0.0',
      isCustom: true,
    });

    // Apply immediately (no reload needed)
    applyPreset(themeId);

    const action = editingThemeId ? 'updated' : 'saved';
    toast.success(`Theme "${themeName}" ${action} and applied!`);
    setSaving(false);
  };

  const handleReset = () => {
    if (
      !window.confirm(
        'Are you sure you want to reset to the default theme? This will remove all customizations.'
      )
    ) {
      return;
    }

    // Clear active preset and any overrides
    localStorage.removeItem('theme-overrides');
    applyPreset(null);
    setThemeColors(buildColorsFromTokens());
    setEditingThemeId(null);
    toast.success('Theme reset to Goose Classic!');
  };

  const handleEditTheme = (preset: ThemePreset) => {
    // Merge preset colors on top of defaults so every variable has a value
    setThemeColors(buildColorsFromTokens(preset));
    setThemeName(preset.name);
    setThemeDescription(preset.description || '');
    setEditingThemeId(preset.id);
    setActiveTab('customize');
    toast.info(`Editing "${preset.name}"`);
  };

  const groupedVariables = COLOR_VARIABLES.reduce(
    (acc, variable) => {
      if (!acc[variable.category]) {
        acc[variable.category] = [];
      }
      acc[variable.category].push(variable);
      return acc;
    },
    {} as Record<string, typeof COLOR_VARIABLES>
  );

  return (
    <Dialog open onOpenChange={onClose}>
      <DialogContent className="!max-w-none !w-screen !h-screen !top-0 !left-0 !translate-x-0 !translate-y-0 !m-0 !rounded-none !p-0 flex flex-col !inset-0">
        <DialogHeader className="px-6 pt-12 pb-4 border-b border-border-primary flex-shrink-0">
          <div className="flex items-center justify-between mb-4">
            <div>
              <DialogTitle className="text-lg">Theme Builder</DialogTitle>
              <DialogDescription className="mt-1">
                Create your perfect theme with presets or custom colors
              </DialogDescription>
            </div>
            <div className="flex gap-2">
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="outline"
                    onClick={handleReset}
                    disabled={saving}
                    size="sm"
                    shape="round"
                  >
                    <RotateCcw className="w-4 h-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>Reset to Default Theme</TooltipContent>
              </Tooltip>

              <Tooltip>
                <TooltipTrigger asChild>
                  <Button onClick={handleSave} disabled={saving} size="sm" shape="round">
                    <Save className="w-4 h-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>{saving ? 'Saving...' : 'Save Theme'}</TooltipContent>
              </Tooltip>
            </div>
          </div>

          <div className="flex items-center gap-4">
            <Tabs
              value={activeTab}
              onValueChange={(v) => setActiveTab(v as 'presets' | 'customize')}
            >
              <TabsList className="inline-flex">
                <TabsTrigger value="presets" className="flex items-center gap-2">
                  <Palette className="w-4 h-4" />
                  <span>Theme Presets</span>
                </TabsTrigger>
                <TabsTrigger value="customize" className="flex items-center gap-2">
                  <Paintbrush className="w-4 h-4" />
                  <span>Custom Colors</span>
                </TabsTrigger>
              </TabsList>
            </Tabs>

            {activeTab === 'customize' && (
              <div className="text-sm text-text-secondary bg-background-secondary px-3 py-1.5 rounded-md">
                Editing:{' '}
                <span className="font-medium text-text-primary capitalize">{activeMode} Mode</span>
              </div>
            )}
          </div>
        </DialogHeader>

        <Tabs
          value={activeTab}
          onValueChange={(v) => setActiveTab(v as 'presets' | 'customize')}
          className="flex-1 flex flex-col overflow-hidden"
        >
          <div className="hidden">
            <TabsList>
              <TabsTrigger value="presets">Theme Presets</TabsTrigger>
              <TabsTrigger value="customize">Custom Colors</TabsTrigger>
            </TabsList>
          </div>

          {/* Presets Tab */}
          <TabsContent value="presets" className="flex-1 overflow-auto px-6 py-4">
            <PresetGallery onApply={onClose} onEdit={handleEditTheme} />
          </TabsContent>

          {/* Customize Tab */}
          <TabsContent
            value="customize"
            className="flex-1 overflow-hidden flex flex-col px-6 py-4"
          >
            {/* Split Panel Layout */}
            <div className="flex gap-4 h-full">
              {/* Left Panel: Color Pickers (40%) */}
              <div className="w-[40%] overflow-auto pr-2 space-y-6">
                {/* Custom Theme Info Card */}
                <div className="border-2 border-border-primary rounded-lg p-4 bg-background-secondary space-y-3">
                  <div className="grid grid-cols-4 h-24 rounded border border-border-primary overflow-hidden">
                    <div
                      style={{
                        backgroundColor:
                          themeColors[activeMode]['color-background-primary'] || '#000000',
                      }}
                    />
                    <div
                      style={{
                        backgroundColor:
                          themeColors[activeMode]['color-background-secondary'] || '#000000',
                      }}
                    />
                    <div
                      style={{
                        backgroundColor:
                          themeColors[activeMode]['color-text-primary'] || '#000000',
                      }}
                    />
                    <div
                      style={{
                        backgroundColor:
                          themeColors[activeMode]['color-background-inverse'] || '#000000',
                      }}
                    />
                  </div>

                  <div>
                    <label className="text-xs text-text-secondary mb-1 block">Theme Name</label>
                    <input
                      type="text"
                      value={themeName}
                      onChange={(e) => setThemeName(e.target.value)}
                      className="w-full px-3 py-2 text-sm border border-border-primary rounded bg-background-primary text-text-primary font-medium"
                      placeholder="My Custom Theme"
                    />
                  </div>

                  <div>
                    <label className="text-xs text-text-secondary mb-1 block">
                      Description (Optional)
                    </label>
                    <input
                      type="text"
                      value={themeDescription}
                      onChange={(e) => setThemeDescription(e.target.value)}
                      className="w-full px-3 py-2 text-sm border border-border-primary rounded bg-background-primary text-text-secondary"
                      placeholder="Describe your theme..."
                    />
                  </div>
                </div>

                {Object.entries(groupedVariables).map(([category, variables]) => (
                  <div key={category} className="space-y-3">
                    <h3 className="text-sm font-semibold text-text-primary capitalize">
                      {category} Colors
                    </h3>
                    <div className="space-y-3">
                      {variables.map((variable) => {
                        const currentColor =
                          themeColors[activeMode][variable.name] || '#000000';
                        const isSelected = selectedVariable === variable.name;

                        return (
                          <div
                            key={variable.name}
                            className={`p-3 rounded-lg border-2 transition-all cursor-pointer ${
                              isSelected
                                ? 'border-border-secondary bg-background-secondary'
                                : 'border-border-primary hover:border-border-secondary'
                            }`}
                            onClick={() => setSelectedVariable(variable.name)}
                          >
                            <div className="flex items-center justify-between mb-2">
                              <div className="flex-1 mr-3">
                                <label className="text-sm font-medium text-text-primary cursor-pointer block">
                                  {variable.label}
                                </label>
                                {variable.description && (
                                  <p className="text-xs text-text-secondary mt-0.5">
                                    {variable.description}
                                  </p>
                                )}
                              </div>
                              <div
                                className="w-10 h-10 rounded border-2 border-border-primary shadow-sm flex-shrink-0"
                                style={{ backgroundColor: currentColor }}
                              />
                            </div>

                            {isSelected && (
                              <div className="mt-3 space-y-3">
                                <SimpleColorPicker
                                  color={currentColor}
                                  onChange={(color) =>
                                    handleColorChange(variable.name, color)
                                  }
                                />
                                <input
                                  type="text"
                                  value={currentColor}
                                  onChange={(e) =>
                                    handleColorChange(variable.name, e.target.value)
                                  }
                                  className="w-full px-3 py-2 text-sm border border-border-primary rounded bg-background-primary text-text-primary font-mono"
                                  placeholder="#000000"
                                />
                              </div>
                            )}
                          </div>
                        );
                      })}
                    </div>
                  </div>
                ))}
              </div>

              {/* Right Panel: Live Preview (60%) */}
              <div className="w-[60%] overflow-auto pl-4 border-l border-border-primary">
                {selectedVariable ? (
                  <ColorPreview
                    variable={COLOR_VARIABLES.find((v) => v.name === selectedVariable)!}
                    lightColor={themeColors.light[selectedVariable] || '#000000'}
                    darkColor={themeColors.dark[selectedVariable] || '#000000'}
                    currentMode={activeMode}
                    allColors={themeColors[activeMode]}
                  />
                ) : (
                  <div className="flex items-center justify-center h-full">
                    <div className="text-center space-y-4">
                      <Pipette className="w-16 h-16 mx-auto text-text-secondary opacity-50" />
                      <div className="space-y-2">
                        <p className="text-text-primary font-medium">
                          Select a color to preview
                        </p>
                        <p className="text-text-secondary text-sm max-w-xs">
                          Click any color on the left to see where it&apos;s used in the UI
                        </p>
                      </div>
                    </div>
                  </div>
                )}
              </div>
            </div>
          </TabsContent>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}
