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

// Hue spectrum colors (24 stops around the color wheel for maximum precision)
const HUE_COLORS = [
  '#ff0000', // Red (0°)
  '#ff4000', // Red-Orange
  '#ff8000', // Orange (30°)
  '#ffbf00', // Orange-Yellow
  '#ffff00', // Yellow (60°)
  '#bfff00', // Yellow-Lime
  '#80ff00', // Lime (90°)
  '#40ff00', // Lime-Green
  '#00ff00', // Green (120°)
  '#00ff40', // Green-Spring
  '#00ff80', // Spring (150°)
  '#00ffbf', // Spring-Cyan
  '#00ffff', // Cyan (180°)
  '#00bfff', // Cyan-Azure
  '#0080ff', // Azure (210°)
  '#0040ff', // Azure-Blue
  '#0000ff', // Blue (240°)
  '#4000ff', // Blue-Violet
  '#8000ff', // Violet (270°)
  '#bf00ff', // Violet-Purple
  '#ff00ff', // Purple (300°)
  '#ff00bf', // Purple-Magenta
  '#ff0080', // Magenta (330°)
  '#ff0040', // Magenta-Red
];

// Generate saturation/lightness grid for a given hue
function generateSaturationGrid(hueColor: string): string[][] {
  const grid: string[][] = [];
  const rows = 10; // Lightness levels (maximum granularity)
  const cols = 15; // Saturation levels (maximum granularity)
  
  // Parse hue color to HSL
  const hsl = hexToHSL(hueColor);
  
  for (let row = 0; row < rows; row++) {
    const rowColors: string[] = [];
    const lightness = 95 - (row * 9.5); // 95% to 5% in 9.5% steps
    
    for (let col = 0; col < cols; col++) {
      const saturation = col * 7; // 0% to 98% in 7% steps
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

// Helper: Find closest hue from HUE_COLORS for a given color
function findClosestHue(color: string): string {
  const hsl = hexToHSL(color);
  const targetHue = hsl.h;
  
  // Find the closest hue from our predefined colors
  let closestHue = HUE_COLORS[0];
  let minDiff = 360;
  
  HUE_COLORS.forEach(hueColor => {
    const hueHSL = hexToHSL(hueColor);
    let diff = Math.abs(hueHSL.h - targetHue);
    // Handle wrap-around (e.g., 350° is close to 10°)
    if (diff > 180) diff = 360 - diff;
    
    if (diff < minDiff) {
      minDiff = diff;
      closestHue = hueColor;
    }
  });
  
  return closestHue;
}

export function SimpleColorPicker({ color, onChange }: SimpleColorPickerProps) {
  // Initialize with the closest hue to the current color
  const [selectedHue, setSelectedHue] = useState(() => findClosestHue(color));
  const saturationGrid = generateSaturationGrid(selectedHue);

  const handleHueSelect = (hueColor: string) => {
    setSelectedHue(hueColor);
  };

  const handleColorSelect = (selectedColor: string) => {
    onChange(selectedColor);
  };

  return (
    <div className="space-y-3">
      {/* Hue Spectrum Bar */}
      <div>
        <div className="text-xs text-text-secondary mb-1.5">Select Hue</div>
        <div className="grid grid-cols-24 gap-0.5">
          {HUE_COLORS.map((hueColor) => (
            <button
              key={hueColor}
              onClick={() => handleHueSelect(hueColor)}
              className={`aspect-square rounded border transition-all hover:scale-110 ${
                selectedHue === hueColor
                  ? 'border-border-secondary ring-1 ring-ring-primary'
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
        <div className="text-xs text-text-secondary mb-1.5">Select Shade</div>
        <div className="space-y-0.5">
          {saturationGrid.map((row, rowIndex) => (
            <div key={rowIndex} className="grid grid-cols-15 gap-0.5">
              {row.map((gridColor, colIndex) => (
                <button
                  key={`${rowIndex}-${colIndex}`}
                  onClick={() => handleColorSelect(gridColor)}
                  className={`aspect-square rounded border transition-all hover:scale-110 ${
                    color.toLowerCase() === gridColor.toLowerCase()
                      ? 'border-border-secondary ring-1 ring-ring-primary'
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
        <div className="text-xs text-text-secondary mb-1.5">Grayscale</div>
        <div className="grid grid-cols-16 gap-0.5">
          {[
            '#000000', '#111111', '#222222', '#333333', '#444444',
            '#555555', '#666666', '#777777', '#888888', '#999999',
            '#aaaaaa', '#bbbbbb', '#cccccc', '#dddddd', '#eeeeee', '#ffffff',
          ].map((grayColor) => (
            <button
              key={grayColor}
              onClick={() => handleColorSelect(grayColor)}
              className={`aspect-square rounded border transition-all hover:scale-110 ${
                color.toLowerCase() === grayColor.toLowerCase()
                  ? 'border-border-secondary ring-1 ring-ring-primary'
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
