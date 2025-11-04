import { useEffect, useState } from 'react';
import { RotateCcw, Plus } from 'lucide-react';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { isValidHexColor, DEFAULT_THEME_COLOR } from '../../utils/colorUtils';
import { cn } from '../../utils';

interface CustomColorPickerProps {
  value: string;
  onChange: (color: string) => void;
  onReset: () => void;
  className?: string;
}

const PRESET_COLORS = [
  DEFAULT_THEME_COLOR,
  '#13bbaf',
  '#ff4f00',
  '#5c98f9',
  '#91cb80',
  '#f94b4b',
] as const;

export function CustomColorPicker({ value, onChange, onReset, className }: CustomColorPickerProps) {
  const [inputValue, setInputValue] = useState(value);
  const [isValid, setIsValid] = useState(true);
  const [showCustomPicker, setShowCustomPicker] = useState(false);

  useEffect(() => {
    setInputValue(value);
  }, [value]);

  const handleColorChange = (color: string) => {
    setInputValue(color);
    setIsValid(true);
    onChange(color);
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = e.target.value;
    setInputValue(newValue);

    if (isValidHexColor(newValue)) {
      setIsValid(true);
      onChange(newValue);
    } else {
      setIsValid(false);
    }
  };

  const isSelected = (color: string) => inputValue.toLowerCase() === color.toLowerCase();
  const isPresetColor = PRESET_COLORS.some(
    (color) => color.toLowerCase() === inputValue.toLowerCase()
  );

  return (
    <div className={cn('space-y-3', className)}>
      {/* Preset Colors Grid */}
      <div className="space-y-2">
        <label className="text-xs text-text-muted">Accent Color</label>
        <div className="grid grid-cols-6 gap-2" role="group" aria-label="Preset colors">
          {PRESET_COLORS.map((color) => (
            <button
              key={color}
              type="button"
              onClick={() => handleColorChange(color)}
              className={cn(
                'w-full aspect-square rounded-md border-2 transition-all hover:scale-105',
                isSelected(color)
                  ? 'border-background-accent ring-2 ring-background-accent/30 scale-105'
                  : 'border-border-default hover:border-border-strong'
              )}
              style={{ backgroundColor: color }}
              title={color}
              aria-label={`Select color ${color}`}
              aria-pressed={isSelected(color)}
            />
          ))}
        </div>
      </div>

      {/* Custom Color Button */}
      {!showCustomPicker && (
        <Button
          onClick={() => setShowCustomPicker(true)}
          variant="outline"
          size="sm"
          className="w-full flex items-center justify-center gap-2"
          data-testid="show-custom-picker-button"
        >
          <Plus className="h-4 w-4" />
          Custom Color
        </Button>
      )}

      {/* Custom Color Picker (Expanded) */}
      {showCustomPicker && (
        <div className="space-y-3 pt-2 border-t border-border-default" data-testid="custom-picker-expanded">
          <div className="flex gap-2 items-center">
            <input
              type="color"
              value={inputValue}
              onChange={(e) => handleColorChange(e.target.value)}
              className="w-10 h-10 rounded-md cursor-pointer border-2 border-border-default bg-transparent flex-shrink-0"
              title="Pick a color"
              aria-label="Color picker"
            />

            <div className="flex-1 min-w-0">
              <Input
                type="text"
                value={inputValue}
                onChange={handleInputChange}
                placeholder={DEFAULT_THEME_COLOR}
                className={cn(
                  'font-mono text-sm',
                  !isValid && 'border-border-danger bg-background-danger/10 text-text-danger'
                )}
                aria-label="Hex color input"
                aria-invalid={!isValid}
                aria-describedby={!isValid ? 'color-error' : undefined}
              />
              {!isValid && (
                <p id="color-error" className="text-xs text-text-danger mt-1" role="alert">
                  Invalid hex color
                </p>
              )}
            </div>

            <Button
              onClick={onReset}
              variant="ghost"
              size="sm"
              className="px-3 flex-shrink-0"
              title="Reset to default"
              aria-label="Reset color"
            >
              <RotateCcw className="h-4 w-4" />
            </Button>
          </div>

          <Button
            onClick={() => setShowCustomPicker(false)}
            variant="ghost"
            size="sm"
            className="w-full"
            data-testid="hide-custom-picker-button"
          >
            Hide Custom Picker
          </Button>
        </div>
      )}

      {/* Show current custom color indicator if not a preset */}
      {!isPresetColor && !showCustomPicker && (
        <div className="flex items-center gap-2 text-xs text-text-muted">
          <div
            className="w-4 h-4 rounded border border-border-default"
            style={{ backgroundColor: inputValue }}
          />
          <span>Custom: {inputValue}</span>
        </div>
      )}
    </div>
  );
}
