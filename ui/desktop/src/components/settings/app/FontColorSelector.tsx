import React, { useEffect, useState } from 'react';
import { Palette } from 'lucide-react';
import { Button } from '../../ui/button';
import { Input } from '../../ui/input';
import { cn } from '../../../utils';

interface FontColorSelectorProps {
  className?: string;
  hideTitle?: boolean;
}

interface ColorPreset {
  name: string;
  color: string;
  description: string;
}

const COLOR_PRESETS: ColorPreset[] = [
  {
    name: 'Default',
    color: '#ff4f00',
    description: 'Original orange',
  },
  {
    name: 'VS Code Dark+',
    color: '#CE9178',
    description: 'Warm beige for dark mode',
  },
  {
    name: 'IntelliJ Darcula',
    color: '#A9B7C6',
    description: 'Light cyan/gray',
  },
  {
    name: 'Sublime Text',
    color: '#F8F8F2',
    description: 'Off-white, high contrast',
  },
];

const STORAGE_KEY = 'code_font_color';
const DEFAULT_COLOR = '#ff4f00';

const FontColorSelector: React.FC<FontColorSelectorProps> = ({
  className = '',
  hideTitle = false,
}) => {
  const [selectedColor, setSelectedColor] = useState<string>(() => {
    const saved = localStorage.getItem(STORAGE_KEY);
    return saved || DEFAULT_COLOR;
  });

  const [customColor, setCustomColor] = useState<string>(() => {
    const saved = localStorage.getItem(STORAGE_KEY);
    // If saved color is not a preset, it's custom
    if (saved && !COLOR_PRESETS.some((preset) => preset.color === saved)) {
      return saved;
    }
    return DEFAULT_COLOR;
  });

  const [isCustomMode, setIsCustomMode] = useState<boolean>(() => {
    const saved = localStorage.getItem(STORAGE_KEY);
    return saved !== null && !COLOR_PRESETS.some((preset) => preset.color === saved);
  });

  // Apply color to CSS variable whenever it changes
  useEffect(() => {
    const colorToApply = isCustomMode ? customColor : selectedColor;
    document.documentElement.style.setProperty('--code-font-color', colorToApply);
    localStorage.setItem(STORAGE_KEY, colorToApply);
  }, [selectedColor, customColor, isCustomMode]);

  // Ensure CSS variable is set on initial mount
  useEffect(() => {
    const saved = localStorage.getItem(STORAGE_KEY);
    const colorToApply = saved || DEFAULT_COLOR;
    document.documentElement.style.setProperty('--code-font-color', colorToApply);
  }, []);

  const handlePresetClick = (color: string) => {
    setSelectedColor(color);
    setIsCustomMode(false);
    document.documentElement.style.setProperty('--code-font-color', color);
    localStorage.setItem(STORAGE_KEY, color);
  };

  const handleCustomColorChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newColor = e.target.value;
    setCustomColor(newColor);
    setIsCustomMode(true);
    document.documentElement.style.setProperty('--code-font-color', newColor);
    localStorage.setItem(STORAGE_KEY, newColor);
  };

  const handleHexInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newColor = e.target.value;
    // Only update if it's a valid hex color (6 hex digits after #)
    if (/^#[0-9A-Fa-f]{6}$/.test(newColor)) {
      setCustomColor(newColor);
      setIsCustomMode(true);
      document.documentElement.style.setProperty('--code-font-color', newColor);
      localStorage.setItem(STORAGE_KEY, newColor);
    } else {
      // Allow partial input for better UX
      setCustomColor(newColor);
    }
  };

  const isPresetSelected = (presetColor: string) => {
    return !isCustomMode && selectedColor === presetColor;
  };

  return (
    <div className={`${!hideTitle ? 'space-y-3' : 'space-y-2'} ${className}`}>
      {!hideTitle && (
        <div>
          <h3 className="text-text-default text-xs mb-1">Inline Code Font Color</h3>
          <p className="text-xs text-text-muted">
            Choose a color for inline code elements to improve readability
          </p>
        </div>
      )}

      <div className="space-y-2">
        <div className="grid grid-cols-2 gap-2">
          {COLOR_PRESETS.map((preset) => (
            <Button
              key={preset.name}
              onClick={() => handlePresetClick(preset.color)}
              className={cn(
                'flex items-center justify-start gap-2 p-2 rounded-md border transition-colors text-xs h-auto',
                isPresetSelected(preset.color)
                  ? 'bg-background-accent text-text-on-accent border-border-accent hover:!bg-background-accent hover:!text-text-on-accent'
                  : 'border-border-default hover:!bg-background-muted text-text-muted hover:text-text-default'
              )}
              variant="ghost"
              size="sm"
            >
              <div
                className="w-4 h-4 rounded border border-border-default"
                style={{ backgroundColor: preset.color }}
              />
              <div className="flex flex-col items-start">
                <span className="font-medium">{preset.name}</span>
                <span className="text-[10px] opacity-75">{preset.description}</span>
              </div>
            </Button>
          ))}
        </div>

        <div className="flex items-center gap-2 pt-1">
          <div className="flex-1">
            <div className="flex items-center gap-2">
              <Palette className="h-4 w-4 text-text-muted" />
              <label className="text-xs text-text-default font-medium">Custom Color</label>
            </div>
            <Input
              type="color"
              value={customColor}
              onChange={handleCustomColorChange}
              className="mt-1 h-8 w-full cursor-pointer"
              style={{ cursor: 'pointer' }}
            />
          </div>
          {isCustomMode && (
            <div className="flex flex-col items-end gap-1">
              <Input
                type="text"
                value={customColor}
                onChange={handleHexInputChange}
                placeholder="#000000"
                className="h-8 w-24 text-xs font-mono"
                pattern="^#[0-9A-Fa-f]{6}$"
              />
              <span className="text-[10px] text-text-muted">Hex value</span>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default FontColorSelector;

