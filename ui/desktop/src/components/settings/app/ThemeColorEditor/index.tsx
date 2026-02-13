/**
 * ThemeColorEditor Component
 * 
 * Main component for theme customization with color picking,
 * preset themes, and advanced features.
 */

import { useState, useEffect } from 'react';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '../../../ui/dialog';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../../../ui/tabs';
import { Button } from '../../../ui/button';
import { getThemeVariables, saveTheme } from '../../../../api';
import { ThemeColorEditorProps, ThemeColors, ColorMode, COLOR_VARIABLES } from './types';
import { HexColorPicker } from 'react-colorful';
import { toast } from 'react-toastify';
import { PresetGallery } from './ThemeSelector/PresetGallery';

export function ThemeColorEditor({ onClose }: ThemeColorEditorProps) {
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [themeColors, setThemeColors] = useState<ThemeColors>({ light: {}, dark: {} });
  const [activeTab, setActiveTab] = useState<'presets' | 'customize'>('presets');
  const [activeMode, setActiveMode] = useState<ColorMode>('light');
  const [selectedVariable, setSelectedVariable] = useState<string | null>(null);

  // Load current theme variables
  useEffect(() => {
    loadThemeVariables();
  }, []);

  const loadThemeVariables = async () => {
    try {
      setLoading(true);
      const response = await getThemeVariables();
      
      if (response.data?.variables) {
        // Parse light-dark() format into separate light and dark values
        const light: Record<string, string> = {};
        const dark: Record<string, string> = {};
        
        Object.entries(response.data.variables).forEach(([key, value]) => {
          const match = value.match(/light-dark\((.+?),\s*(.+?)\)/);
          if (match) {
            const varName = key.replace('--', '');
            light[varName] = match[1].trim();
            dark[varName] = match[2].trim();
          }
        });
        
        setThemeColors({ light, dark });
      }
    } catch (error) {
      console.error('Failed to load theme variables:', error);
      toast.error('Failed to load theme colors');
    } finally {
      setLoading(false);
    }
  };

  const handleColorChange = (variableName: string, color: string) => {
    setThemeColors(prev => ({
      ...prev,
      [activeMode]: {
        ...prev[activeMode],
        [variableName]: color,
      },
    }));
  };

  const handleSave = async () => {
    try {
      setSaving(true);
      
      // Convert back to CSS format
      const cssLines: string[] = [];
      
      // Light mode
      cssLines.push(':root {');
      Object.entries(themeColors.light).forEach(([key, value]) => {
        cssLines.push(`  --${key}: ${value};`);
      });
      cssLines.push('}');
      cssLines.push('');
      
      // Dark mode
      cssLines.push('.dark {');
      Object.entries(themeColors.dark).forEach(([key, value]) => {
        cssLines.push(`  --${key}: ${value};`);
      });
      cssLines.push('}');
      
      const css = cssLines.join('\n');
      
      await saveTheme({ body: { css } });
      toast.success('Theme saved successfully!');
      
      // Reload the page to apply changes
      window.location.reload();
    } catch (error) {
      console.error('Failed to save theme:', error);
      toast.error('Failed to save theme');
    } finally {
      setSaving(false);
    }
  };

  const handleReset = async () => {
    if (!confirm('Are you sure you want to reset to the default theme? This will remove all customizations.')) {
      return;
    }
    
    try {
      setSaving(true);
      await saveTheme({ body: { css: '' } }); // Empty CSS resets theme
      toast.success('Theme reset successfully!');
      window.location.reload();
    } catch (error) {
      console.error('Failed to reset theme:', error);
      toast.error('Failed to reset theme');
    } finally {
      setSaving(false);
    }
  };

  const groupedVariables = COLOR_VARIABLES.reduce((acc, variable) => {
    if (!acc[variable.category]) {
      acc[variable.category] = [];
    }
    acc[variable.category].push(variable);
    return acc;
  }, {} as Record<string, typeof COLOR_VARIABLES>);

  if (loading) {
    return (
      <Dialog open onOpenChange={onClose}>
        <DialogContent className="max-w-4xl max-h-[90vh]">
          <div className="flex items-center justify-center p-8">
            <div className="text-text-secondary">Loading theme...</div>
          </div>
        </DialogContent>
      </Dialog>
    );
  }

  return (
    <Dialog open onOpenChange={onClose}>
      <DialogContent className="max-w-4xl max-h-[90vh] overflow-hidden flex flex-col">
        <DialogHeader>
          <DialogTitle>Customize Theme</DialogTitle>
        </DialogHeader>

        <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as 'presets' | 'customize')} className="flex-1 flex flex-col overflow-hidden">
          <TabsList className="grid w-full grid-cols-2">
            <TabsTrigger value="presets">Theme Presets</TabsTrigger>
            <TabsTrigger value="customize">Custom Colors</TabsTrigger>
          </TabsList>

          {/* Presets Tab */}
          <TabsContent value="presets" className="flex-1 overflow-auto mt-4">
            <PresetGallery onApply={onClose} />
          </TabsContent>

          {/* Customize Tab */}
          <TabsContent value="customize" className="flex-1 overflow-hidden flex flex-col">
            <Tabs value={activeMode} onValueChange={(v) => setActiveMode(v as ColorMode)} className="flex-1 flex flex-col overflow-hidden">
              <TabsList className="grid w-full grid-cols-2 mb-4">
                <TabsTrigger value="light">Light Mode</TabsTrigger>
                <TabsTrigger value="dark">Dark Mode</TabsTrigger>
              </TabsList>

          <TabsContent value={activeMode} className="flex-1 overflow-auto mt-4">
            <div className="space-y-6 pr-2">
              {Object.entries(groupedVariables).map(([category, variables]) => (
                <div key={category} className="space-y-3">
                  <h3 className="text-sm font-semibold text-text-primary capitalize">
                    {category} Colors
                  </h3>
                  <div className="grid grid-cols-2 gap-4">
                    {variables.map((variable) => {
                      const currentColor = themeColors[activeMode][variable.name] || '#000000';
                      const isSelected = selectedVariable === variable.name;
                      
                      return (
                        <div key={variable.name} className="space-y-2">
                          <div className="flex items-center justify-between">
                            <label className="text-xs text-text-secondary">
                              {variable.label}
                            </label>
                            <div
                              className="w-8 h-8 rounded border-2 border-border-primary cursor-pointer hover:scale-110 transition-transform"
                              style={{ backgroundColor: currentColor }}
                              onClick={() => setSelectedVariable(isSelected ? null : variable.name)}
                            />
                          </div>
                          
                          {isSelected && (
                            <div className="mt-2">
                              <HexColorPicker
                                color={currentColor}
                                onChange={(color) => handleColorChange(variable.name, color)}
                              />
                              <input
                                type="text"
                                value={currentColor}
                                onChange={(e) => handleColorChange(variable.name, e.target.value)}
                                className="mt-2 w-full px-2 py-1 text-xs border border-border-primary rounded bg-background-primary text-text-primary"
                                placeholder="#000000"
                              />
                            </div>
                          )}
                          
                          {variable.description && (
                            <p className="text-xs text-text-secondary opacity-70">
                              {variable.description}
                            </p>
                          )}
                        </div>
                      );
                    })}
                  </div>
                </div>
              ))}
            </div>
              </TabsContent>
            </Tabs>

            <div className="flex justify-between items-center pt-4 border-t border-border-primary mt-4">
              <Button
                variant="outline"
                onClick={handleReset}
                disabled={saving}
              >
                Reset to Default
              </Button>
              
              <div className="flex gap-2">
                <Button
                  variant="outline"
                  onClick={onClose}
                  disabled={saving}
                >
                  Cancel
                </Button>
                <Button
                  onClick={handleSave}
                  disabled={saving}
                >
                  {saving ? 'Saving...' : 'Save Theme'}
                </Button>
              </div>
            </div>
          </TabsContent>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}
