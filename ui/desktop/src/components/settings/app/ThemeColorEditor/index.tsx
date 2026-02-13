/**
 * ThemeColorEditor Component
 * 
 * Main component for theme customization with color picking,
 * preset themes, and advanced features.
 */

import { useState, useEffect } from 'react';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from '../../../ui/dialog';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../../../ui/tabs';
import { Button } from '../../../ui/button';
import { getThemeVariables, saveTheme } from '../../../../api';
import { ThemeColorEditorProps, ThemeColors, ColorMode, COLOR_VARIABLES, ColorVariable } from './types';
import { HexColorPicker } from 'react-colorful';
import { toast } from 'react-toastify';
import { PresetGallery } from './ThemeSelector/PresetGallery';
import { ColorPreview } from './Preview/ColorPreview';
import { RotateCcw, Save } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipTrigger } from '../../../ui/Tooltip';

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
                  <Button
                    onClick={handleSave}
                    disabled={saving}
                    size="sm"
                    shape="round"
                  >
                    <Save className="w-4 h-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>{saving ? 'Saving...' : 'Save Theme'}</TooltipContent>
              </Tooltip>
            </div>
          </div>
          
          <div className="flex items-center gap-4">
            <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as 'presets' | 'customize')} className="flex-1">
              <TabsList className="grid w-full grid-cols-2 max-w-md">
                <TabsTrigger value="presets">Theme Presets</TabsTrigger>
                <TabsTrigger value="customize">Custom Colors</TabsTrigger>
              </TabsList>
            </Tabs>
            
            {activeTab === 'customize' && (
              <Tabs value={activeMode} onValueChange={(v) => setActiveMode(v as ColorMode)}>
                <TabsList className="grid grid-cols-2">
                  <TabsTrigger value="light">Light Mode</TabsTrigger>
                  <TabsTrigger value="dark">Dark Mode</TabsTrigger>
                </TabsList>
              </Tabs>
            )}
          </div>
        </DialogHeader>

        <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as 'presets' | 'customize')} className="flex-1 flex flex-col overflow-hidden">
          <div className="hidden">
            <TabsList>
              <TabsTrigger value="presets">Theme Presets</TabsTrigger>
              <TabsTrigger value="customize">Custom Colors</TabsTrigger>
            </TabsList>
          </div>

          {/* Presets Tab */}
          <TabsContent value="presets" className="flex-1 overflow-auto px-6 py-4">
            <PresetGallery onApply={onClose} />
          </TabsContent>

          {/* Customize Tab */}
          <TabsContent value="customize" className="flex-1 overflow-hidden flex flex-col px-6 py-4">
            <Tabs value={activeMode} onValueChange={(v) => setActiveMode(v as ColorMode)} className="flex-1 flex flex-col overflow-hidden">
              <div className="hidden">
                <TabsList>
                  <TabsTrigger value="light">Light Mode</TabsTrigger>
                  <TabsTrigger value="dark">Dark Mode</TabsTrigger>
                </TabsList>
              </div>

          <TabsContent value={activeMode} className="flex-1 overflow-hidden">
            {/* Split Panel Layout */}
            <div className="flex gap-4 h-full">
              {/* Left Panel: Color Pickers (40%) */}
              <div className="w-[40%] overflow-auto pr-2 space-y-6">
                {Object.entries(groupedVariables).map(([category, variables]) => (
                  <div key={category} className="space-y-3">
                    <h3 className="text-sm font-semibold text-text-primary capitalize">
                      {category} Colors
                    </h3>
                    <div className="space-y-3">
                      {variables.map((variable) => {
                        const currentColor = themeColors[activeMode][variable.name] || '#000000';
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
                              <label className="text-xs font-medium text-text-primary cursor-pointer">
                                {variable.label}
                              </label>
                              <div
                                className="w-10 h-10 rounded border-2 border-border-primary shadow-sm"
                                style={{ backgroundColor: currentColor }}
                              />
                            </div>
                            
                            {isSelected && (
                              <div className="mt-3 space-y-2">
                                <HexColorPicker
                                  color={currentColor}
                                  onChange={(color) => handleColorChange(variable.name, color)}
                                  style={{ width: '100%' }}
                                />
                                <input
                                  type="text"
                                  value={currentColor}
                                  onChange={(e) => handleColorChange(variable.name, e.target.value)}
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
                    variable={COLOR_VARIABLES.find(v => v.name === selectedVariable)!}
                    lightColor={themeColors.light[selectedVariable] || '#000000'}
                    darkColor={themeColors.dark[selectedVariable] || '#000000'}
                    currentMode={activeMode}
                  />
                ) : (
                  <div className="flex items-center justify-center h-full">
                    <div className="text-center space-y-2">
                      <div className="text-4xl">🎨</div>
                      <p className="text-text-primary font-medium">Select a color to preview</p>
                      <p className="text-text-secondary text-sm max-w-xs">
                        Click any color on the left to see where it's used in the UI
                      </p>
                    </div>
                  </div>
                )}
              </div>
            </div>
              </TabsContent>
            </Tabs>
          </TabsContent>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}
