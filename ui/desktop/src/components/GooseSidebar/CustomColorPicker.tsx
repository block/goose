import { useEffect, useState, useCallback } from 'react';
import { Plus } from 'lucide-react';
import { isValidHexColor, DEFAULT_THEME_COLOR } from '../../utils/colorUtils';
import { cn } from '../../utils';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/dialog';
import { Button } from '../ui/button';
import { Input } from '../ui/input';

interface CustomColorPickerProps {
  value: string;
  onChange: (color: string) => void;
  onReset: () => void;
  className?: string;
}

const DEFAULT_PRESET_COLORS = [
  DEFAULT_THEME_COLOR,
  '#13bbaf',
  '#ff4f00',
  '#5c98f9',
  '#91cb80',
  '#f94b4b',
] as const;

const CUSTOM_COLORS_KEY = 'custom_accent_colors';
const MAX_COLORS = 12; // Maximum colors to show in grid

export function CustomColorPicker({ value, onChange, onReset, className }: CustomColorPickerProps) {
  const [customColors, setCustomColors] = useState<string[]>([]);
  const [showColorDialog, setShowColorDialog] = useState(false);
  const [tempColor, setTempColor] = useState(DEFAULT_THEME_COLOR);
  const [tempHexInput, setTempHexInput] = useState(DEFAULT_THEME_COLOR);
  const [isValid, setIsValid] = useState(true);

  // Load custom colors from localStorage on mount
  useEffect(() => {
    if (typeof window !== 'undefined' && window.localStorage) {
      try {
        const saved = localStorage.getItem(CUSTOM_COLORS_KEY);
        if (saved) {
          const parsed = JSON.parse(saved);
          if (Array.isArray(parsed)) {
            setCustomColors(parsed);
          }
        }
      } catch (error) {
        console.error('Failed to load custom colors:', error);
      }
    }
  }, []);

  // Save custom colors to localStorage
  const saveCustomColors = useCallback((colors: string[]) => {
    if (typeof window !== 'undefined' && window.localStorage) {
      try {
        localStorage.setItem(CUSTOM_COLORS_KEY, JSON.stringify(colors));
      } catch (error) {
        console.error('Failed to save custom colors:', error);
      }
    }
  }, []);

  // Combine default presets with custom colors
  const allColors = [
    ...DEFAULT_PRESET_COLORS,
    ...customColors.filter(
      (color) =>
        !DEFAULT_PRESET_COLORS.some((preset) => preset.toLowerCase() === color.toLowerCase())
    ),
  ].slice(0, MAX_COLORS);

  const handleColorSelect = (color: string) => {
    onChange(color);
  };

  const handleOpenDialog = () => {
    setTempColor(value);
    setTempHexInput(value);
    setIsValid(true);
    setShowColorDialog(true);
  };

  const handleColorPickerChange = (color: string) => {
    setTempColor(color);
    setTempHexInput(color);
    setIsValid(true);
  };

  const handleHexInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = e.target.value;
    setTempHexInput(newValue);

    if (isValidHexColor(newValue)) {
      setIsValid(true);
      setTempColor(newValue);
    } else {
      setIsValid(false);
    }
  };

  const handleAddColor = () => {
    if (!isValid) return;

    // Check if color already exists in presets or custom colors
    const colorExists =
      DEFAULT_PRESET_COLORS.some((c) => c.toLowerCase() === tempColor.toLowerCase()) ||
      customColors.some((c) => c.toLowerCase() === tempColor.toLowerCase());

    if (!colorExists) {
      const newCustomColors = [...customColors, tempColor];
      setCustomColors(newCustomColors);
      saveCustomColors(newCustomColors);
    }

    // Apply the color
    onChange(tempColor);
    setShowColorDialog(false);
  };

  const handleCancel = () => {
    setShowColorDialog(false);
    setTempColor(value);
    setTempHexInput(value);
    setIsValid(true);
  };

  const isSelected = (color: string) => value.toLowerCase() === color.toLowerCase();

  return (
    <div className={cn(className)}>
      {/* Color Flex Layout */}
      <div className="flex flex-wrap gap-1 p-1" role="group" aria-label="Accent colors">
        {allColors.map((color) => (
          <button
            key={color}
            type="button"
            onClick={() => handleColorSelect(color)}
            className={cn(
              'w-8 h-8 rounded border-2 transition-all hover:scale-110 flex-shrink-0',
              isSelected(color)
                ? 'border-background-accent ring-2 ring-background-accent/30 scale-110'
                : 'border-border-default hover:border-border-strong'
            )}
            style={{ backgroundColor: color }}
            title={color}
            aria-label={`Select color ${color}`}
            aria-pressed={isSelected(color)}
          />
        ))}

        {/* Add Custom Color Button */}
        {allColors.length < MAX_COLORS && (
          <button
            type="button"
            onClick={handleOpenDialog}
            className="w-8 h-8 rounded border-2 border-dashed border-border-default hover:border-border-strong transition-all hover:scale-110 flex items-center justify-center flex-shrink-0"
            title="Add custom color"
            aria-label="Add custom color"
            data-testid="add-custom-color-button"
          >
            <Plus className="h-4 w-4 text-text-muted" />
          </button>
        )}
      </div>

      {/* Custom Color Dialog */}
      <Dialog open={showColorDialog} onOpenChange={setShowColorDialog}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Add Custom Color</DialogTitle>
            <DialogDescription>
              Choose a custom accent color to add to your palette
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-4">
            <div className="flex gap-3 items-start">
              <input
                type="color"
                value={tempColor}
                onChange={(e) => handleColorPickerChange(e.target.value)}
                className="w-16 h-16 rounded-md cursor-pointer border-2 border-border-default bg-transparent flex-shrink-0"
                title="Pick a color"
                aria-label="Color picker"
              />

              <div className="flex-1 space-y-2">
                <label htmlFor="hex-input" className="text-sm font-medium text-text-default">
                  Hex Color
                </label>
                <Input
                  id="hex-input"
                  type="text"
                  value={tempHexInput}
                  onChange={handleHexInputChange}
                  placeholder={DEFAULT_THEME_COLOR}
                  className={cn(
                    'font-mono',
                    !isValid && 'border-border-danger bg-background-danger/10 text-text-danger'
                  )}
                  aria-label="Hex color input"
                  aria-invalid={!isValid}
                  aria-describedby={!isValid ? 'color-error' : undefined}
                />
                {!isValid && (
                  <p id="color-error" className="text-xs text-text-danger" role="alert">
                    Invalid hex color (e.g., #ff0000)
                  </p>
                )}
              </div>
            </div>

            {/* Preview */}
            <div className="space-y-2">
              <label className="text-sm font-medium text-text-default">Preview</label>
              <div
                className="w-full h-12 rounded-md border-2 border-border-default"
                style={{ backgroundColor: isValid ? tempColor : '#cccccc' }}
              />
            </div>
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={handleCancel}>
              Cancel
            </Button>
            <Button onClick={handleAddColor} disabled={!isValid}>
              Add Color
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
