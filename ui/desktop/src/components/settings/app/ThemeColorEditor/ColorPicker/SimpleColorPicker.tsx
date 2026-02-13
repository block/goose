/**
 * SimpleColorPicker Component
 * 
 * A simplified color picker with discrete color stops instead of gradients.
 * Uses a spectrum bar of color squares and a saturation/lightness grid.
 */

import { useState } from 'react';

interface SimpleColorPickerProps {
  color: string;
  onChange: (color: string) => void;
}

// Hue spectrum colors (12 stops around the color wheel)
const HUE_COLORS = [
  '#ff0000', // Red
  '#ff8000', // Orange
  '#ffff00', // Yellow
  '#80ff00', // Yellow-Green
  '#00ff00', // Green
  '#00ff80', // Green-Cyan
  '#00ffff', // Cyan
  '#0080ff', // Cyan-Blue
  '#0000ff', // Blue
  '#8000ff', // Blue-Purple
  '#ff00ff', // Purple
  '#ff0080', // Purple-Red
];

// Generate saturation/lightness grid for a given hue
function generateSaturationGrid(hueColor: string): string[][] {
  const grid: string[][] = [];
  const rows = 5; // Lightness levels
  const cols = 5; // Saturation levels
  
  // Parse hue color to HSL
  const hsl = hexToHSL(hueColor);
  
  for (let row = 0; row < rows; row++) {
    const rowColors: string[] = [];
    const lightness = 90 - (row * 20); // 90%, 70%, 50%, 30%, 10%
    
    for (let col = 0; col < cols; col++) {
      const saturation = col * 25; // 0%, 25%, 50%, 75%, 100%
      const color = hslToHex(hsl.h, saturation, lightness);
      rowColors.push(color);
    }
    grid.push(rowColors);
  }
  
  return grid;
}

// Helper: Convert hex to HSL
function hexToHSL(hex: string): { h: number; s: number; l: number } {
  const r = parseInt(hex.slice(1, 3), 16) / 255;
  const g = parseInt(hex.slice(3, 5), 16) / 255;
  const b = parseInt(hex.slice(5, 7), 16) / 255;

  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  let h = 0;
  let s = 0;
  const l = (max + min) / 2;

  if (max !== min) {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);

    switch (max) {
      case r:
        h = ((g - b) / d + (g < b ? 6 : 0)) / 6;
        break;
      case g:
        h = ((b - r) / d + 2) / 6;
        break;
      case b:
        h = ((r - g) / d + 4) / 6;
        break;
    }
  }

  return { h: h * 360, s: s * 100, l: l * 100 };
}

// Helper: Convert HSL to hex
function hslToHex(h: number, s: number, l: number): string {
  h = h / 360;
  s = s / 100;
  l = l / 100;

  let r, g, b;

  if (s === 0) {
    r = g = b = l;
  } else {
    const hue2rgb = (p: number, q: number, t: number) => {
      if (t < 0) t += 1;
      if (t > 1) t -= 1;
      if (t < 1 / 6) return p + (q - p) * 6 * t;
      if (t < 1 / 2) return q;
      if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
      return p;
    };

    const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
    const p = 2 * l - q;

    r = hue2rgb(p, q, h + 1 / 3);
    g = hue2rgb(p, q, h);
    b = hue2rgb(p, q, h - 1 / 3);
  }

  const toHex = (x: number) => {
    const hex = Math.round(x * 255).toString(16);
    return hex.length === 1 ? '0' + hex : hex;
  };

  return `#${toHex(r)}${toHex(g)}${toHex(b)}`;
}

export function SimpleColorPicker({ color, onChange }: SimpleColorPickerProps) {
  const [selectedHue, setSelectedHue] = useState(HUE_COLORS[0]);
  const saturationGrid = generateSaturationGrid(selectedHue);

  const handleHueSelect = (hueColor: string) => {
    setSelectedHue(hueColor);
  };

  const handleColorSelect = (selectedColor: string) => {
    onChange(selectedColor);
  };

  return (
    <div className="space-y-4">
      {/* Hue Spectrum Bar */}
      <div>
        <div className="text-xs text-text-secondary mb-2">Select Hue</div>
        <div className="grid grid-cols-12 gap-1">
          {HUE_COLORS.map((hueColor) => (
            <button
              key={hueColor}
              onClick={() => handleHueSelect(hueColor)}
              className={`w-full aspect-square rounded border-2 transition-all hover:scale-110 ${
                selectedHue === hueColor
                  ? 'border-border-secondary ring-2 ring-ring-primary'
                  : 'border-border-primary'
              }`}
              style={{ backgroundColor: hueColor }}
              aria-label={`Select ${hueColor} hue`}
            />
          ))}
        </div>
      </div>

      {/* Saturation/Lightness Grid */}
      <div>
        <div className="text-xs text-text-secondary mb-2">Select Shade</div>
        <div className="space-y-1">
          {saturationGrid.map((row, rowIndex) => (
            <div key={rowIndex} className="grid grid-cols-5 gap-1">
              {row.map((gridColor, colIndex) => (
                <button
                  key={`${rowIndex}-${colIndex}`}
                  onClick={() => handleColorSelect(gridColor)}
                  className={`w-full aspect-square rounded border-2 transition-all hover:scale-110 ${
                    color.toLowerCase() === gridColor.toLowerCase()
                      ? 'border-border-secondary ring-2 ring-ring-primary'
                      : 'border-border-primary'
                  }`}
                  style={{ backgroundColor: gridColor }}
                  aria-label={`Select ${gridColor}`}
                />
              ))}
            </div>
          ))}
        </div>
      </div>

      {/* Grayscale Row */}
      <div>
        <div className="text-xs text-text-secondary mb-2">Grayscale</div>
        <div className="grid grid-cols-6 gap-1">
          {['#000000', '#333333', '#666666', '#999999', '#cccccc', '#ffffff'].map((grayColor) => (
            <button
              key={grayColor}
              onClick={() => handleColorSelect(grayColor)}
              className={`w-full aspect-square rounded border-2 transition-all hover:scale-110 ${
                color.toLowerCase() === grayColor.toLowerCase()
                  ? 'border-border-secondary ring-2 ring-ring-primary'
                  : 'border-border-primary'
              }`}
              style={{ backgroundColor: grayColor }}
              aria-label={`Select ${grayColor}`}
            />
          ))}
        </div>
      </div>
    </div>
  );
}
