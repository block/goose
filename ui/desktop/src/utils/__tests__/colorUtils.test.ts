import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import {
  hexToRgb,
  rgbToHex,
  rgbToHsl,
  hslToRgb,
  generateNeutralScale,
  generateAccentColors,
  generateSemanticColors,
  generateThemePalette,
  applyCustomTheme,
  resetThemeColors,
  isValidHexColor,
  DEFAULT_THEME_COLOR,
} from '../colorUtils';

describe('colorUtils', () => {
  describe('DEFAULT_THEME_COLOR', () => {
    it('should be a valid hex color', () => {
      expect(DEFAULT_THEME_COLOR).toBe('#32353b');
      expect(isValidHexColor(DEFAULT_THEME_COLOR)).toBe(true);
    });
  });

  describe('hexToRgb', () => {
    it('should convert valid hex to RGB', () => {
      expect(hexToRgb('#ffffff')).toEqual({ r: 255, g: 255, b: 255 });
      expect(hexToRgb('#000000')).toEqual({ r: 0, g: 0, b: 0 });
      expect(hexToRgb('#ff0000')).toEqual({ r: 255, g: 0, b: 0 });
      expect(hexToRgb('#32353b')).toEqual({ r: 50, g: 53, b: 59 });
    });

    it('should handle hex without # prefix', () => {
      expect(hexToRgb('ffffff')).toEqual({ r: 255, g: 255, b: 255 });
    });

    it('should handle uppercase hex', () => {
      expect(hexToRgb('#FF00FF')).toEqual({ r: 255, g: 0, b: 255 });
    });

    it('should return null for invalid hex', () => {
      expect(hexToRgb('invalid')).toBeNull();
      expect(hexToRgb('#fff')).toBeNull(); // Too short
      expect(hexToRgb('#gggggg')).toBeNull(); // Invalid characters
      expect(hexToRgb('')).toBeNull();
    });
  });

  describe('rgbToHex', () => {
    it('should convert RGB to hex', () => {
      expect(rgbToHex(255, 255, 255)).toBe('#ffffff');
      expect(rgbToHex(0, 0, 0)).toBe('#000000');
      expect(rgbToHex(255, 0, 0)).toBe('#ff0000');
      expect(rgbToHex(50, 53, 59)).toBe('#32353b');
    });

    it('should pad single digit values', () => {
      expect(rgbToHex(1, 2, 3)).toBe('#010203');
    });
  });

  describe('rgbToHsl', () => {
    it('should convert RGB to HSL', () => {
      const white = rgbToHsl(255, 255, 255);
      expect(white.l).toBeCloseTo(100, 0);

      const black = rgbToHsl(0, 0, 0);
      expect(black.l).toBeCloseTo(0, 0);

      const red = rgbToHsl(255, 0, 0);
      expect(red.h).toBeCloseTo(0, 0);
      expect(red.s).toBeCloseTo(100, 0);
    });

    it('should handle grayscale colors', () => {
      const gray = rgbToHsl(128, 128, 128);
      expect(gray.s).toBeCloseTo(0, 0);
      expect(gray.l).toBeCloseTo(50, 0);
    });
  });

  describe('hslToRgb', () => {
    it('should convert HSL to RGB', () => {
      expect(hslToRgb(0, 0, 100)).toEqual({ r: 255, g: 255, b: 255 });
      expect(hslToRgb(0, 0, 0)).toEqual({ r: 0, g: 0, b: 0 });
      expect(hslToRgb(0, 100, 50)).toEqual({ r: 255, g: 0, b: 0 });
    });

    it('should handle grayscale', () => {
      const gray = hslToRgb(0, 0, 50);
      expect(gray.r).toBeCloseTo(gray.g, 0);
      expect(gray.g).toBeCloseTo(gray.b, 0);
    });
  });

  describe('isValidHexColor', () => {
    it('should validate correct hex colors', () => {
      expect(isValidHexColor('#ffffff')).toBe(true);
      expect(isValidHexColor('#000000')).toBe(true);
      expect(isValidHexColor('#FF00FF')).toBe(true);
      expect(isValidHexColor('#32353b')).toBe(true);
    });

    it('should reject invalid hex colors', () => {
      expect(isValidHexColor('ffffff')).toBe(false); // Missing #
      expect(isValidHexColor('#fff')).toBe(false); // Too short
      expect(isValidHexColor('#gggggg')).toBe(false); // Invalid chars
      expect(isValidHexColor('')).toBe(false);
      expect(isValidHexColor('#12345')).toBe(false); // Wrong length
      expect(isValidHexColor('#1234567')).toBe(false); // Too long
    });
  });

  describe('generateNeutralScale', () => {
    it('should generate 11 neutral shades', () => {
      const neutrals = generateNeutralScale('#32353b');
      const keys = Object.keys(neutrals);

      expect(keys).toHaveLength(11);
      expect(keys).toEqual([
        '50',
        '100',
        '200',
        '300',
        '400',
        '500',
        '600',
        '700',
        '800',
        '900',
        '950',
      ]);
    });

    it('should generate valid hex colors', () => {
      const neutrals = generateNeutralScale('#32353b');

      Object.values(neutrals).forEach((color) => {
        expect(color).toMatch(/^#[0-9a-f]{6}$/i);
      });
    });

    it('should generate progressively darker shades', () => {
      const neutrals = generateNeutralScale('#32353b');
      const rgb50 = hexToRgb(neutrals['50'])!;
      const rgb950 = hexToRgb(neutrals['950'])!;

      // 50 should be lighter than 950
      expect(rgb50.r + rgb50.g + rgb50.b).toBeGreaterThan(rgb950.r + rgb950.g + rgb950.b);
    });

    it('should handle invalid color gracefully', () => {
      const neutrals = generateNeutralScale('invalid');
      expect(Object.keys(neutrals)).toHaveLength(0);
    });
  });

  describe('generateAccentColors', () => {
    it('should generate base, light, and dark variants', () => {
      const accents = generateAccentColors('#32353b');

      expect(accents).toHaveProperty('base');
      expect(accents).toHaveProperty('light');
      expect(accents).toHaveProperty('dark');
      expect(accents.base).toBe('#32353b');
    });

    it('should generate valid hex colors', () => {
      const accents = generateAccentColors('#ff0000');

      expect(isValidHexColor(accents.base)).toBe(true);
      expect(isValidHexColor(accents.light)).toBe(true);
      expect(isValidHexColor(accents.dark)).toBe(true);
    });

    it('should handle invalid color gracefully', () => {
      const accents = generateAccentColors('invalid');
      expect(Object.keys(accents)).toHaveLength(0);
    });
  });

  describe('generateSemanticColors', () => {
    it('should generate all semantic color categories', () => {
      const semantic = generateSemanticColors('#32353b');

      expect(semantic).toHaveProperty('success');
      expect(semantic).toHaveProperty('danger');
      expect(semantic).toHaveProperty('warning');
      expect(semantic).toHaveProperty('info');
    });

    it('should generate two shades per category', () => {
      const semantic = generateSemanticColors('#32353b');

      expect(semantic.success).toHaveProperty('100');
      expect(semantic.success).toHaveProperty('200');
      expect(semantic.danger).toHaveProperty('100');
      expect(semantic.danger).toHaveProperty('200');
      expect(semantic.warning).toHaveProperty('100');
      expect(semantic.warning).toHaveProperty('200');
      expect(semantic.info).toHaveProperty('100');
      expect(semantic.info).toHaveProperty('200');
    });

    it('should generate valid hex colors', () => {
      const semantic = generateSemanticColors('#32353b');

      Object.values(semantic).forEach((category) => {
        Object.values(category).forEach((color) => {
          expect(isValidHexColor(color)).toBe(true);
        });
      });
    });
  });

  describe('generateThemePalette', () => {
    it('should generate complete theme palette', () => {
      const palette = generateThemePalette('#32353b');

      expect(palette).toHaveProperty('neutrals');
      expect(palette).toHaveProperty('accent');
      expect(palette).toHaveProperty('semantic');
    });

    it('should generate consistent palette structure', () => {
      const palette = generateThemePalette('#ff0000');

      expect(Object.keys(palette.neutrals)).toHaveLength(11);
      expect(Object.keys(palette.accent)).toHaveLength(3);
      expect(Object.keys(palette.semantic)).toHaveLength(4);
    });
  });

  describe('applyCustomTheme', () => {
    beforeEach(() => {
      // Create a mock document.documentElement
      const mockSetProperty = vi.fn();
      Object.defineProperty(document, 'documentElement', {
        value: {
          style: {
            setProperty: mockSetProperty,
          },
        },
        writable: true,
        configurable: true,
      });
    });

    afterEach(() => {
      vi.clearAllMocks();
    });

    it('should apply neutral colors to CSS variables', () => {
      applyCustomTheme('#32353b', false);

      const setProperty = document.documentElement.style.setProperty;
      expect(setProperty).toHaveBeenCalledWith(
        expect.stringMatching(/--color-neutral-\d+/),
        expect.stringMatching(/^#[0-9a-f]{6}$/i)
      );
    });

    it('should apply accent color for light mode', () => {
      applyCustomTheme('#32353b', false);

      const setProperty = document.documentElement.style.setProperty;
      const accentCalls = vi
        .mocked(setProperty)
        .mock.calls.filter((call) => call[0] === '--color-accent');
      expect(accentCalls.length).toBeGreaterThan(0);
    });

    it('should apply accent color for dark mode', () => {
      applyCustomTheme('#32353b', true);

      const setProperty = document.documentElement.style.setProperty;
      const accentCalls = vi
        .mocked(setProperty)
        .mock.calls.filter((call) => call[0] === '--color-accent');
      expect(accentCalls.length).toBeGreaterThan(0);
    });

    it('should apply semantic colors', () => {
      applyCustomTheme('#32353b', false);

      const setProperty = document.documentElement.style.setProperty;
      expect(setProperty).toHaveBeenCalledWith('--color-red-100', expect.any(String));
      expect(setProperty).toHaveBeenCalledWith('--color-green-100', expect.any(String));
      expect(setProperty).toHaveBeenCalledWith('--color-yellow-100', expect.any(String));
      expect(setProperty).toHaveBeenCalledWith('--color-blue-100', expect.any(String));
    });
  });

  describe('resetThemeColors', () => {
    beforeEach(() => {
      const mockRemoveProperty = vi.fn();
      Object.defineProperty(document, 'documentElement', {
        value: {
          style: {
            removeProperty: mockRemoveProperty,
          },
        },
        writable: true,
        configurable: true,
      });
    });

    afterEach(() => {
      vi.clearAllMocks();
    });

    it('should remove all custom color properties', () => {
      resetThemeColors();

      const removeProperty = document.documentElement.style.removeProperty;
      expect(removeProperty).toHaveBeenCalledWith('--color-accent');
      expect(removeProperty).toHaveBeenCalledWith('--color-neutral-50');
      expect(removeProperty).toHaveBeenCalledWith('--color-red-100');
      expect(removeProperty).toHaveBeenCalledWith('--color-green-100');
    });

    it('should remove all neutral scale properties', () => {
      resetThemeColors();

      const removeProperty = document.documentElement.style.removeProperty;
      const neutralCalls = vi
        .mocked(removeProperty)
        .mock.calls.filter((call) => call[0].startsWith('--color-neutral-'));
      expect(neutralCalls.length).toBe(11); // 50, 100, 200, ..., 950
    });
  });

  describe('Color conversion round-trip', () => {
    it('should maintain color integrity through conversions', () => {
      const originalHex = '#32353b';
      const rgb = hexToRgb(originalHex);
      expect(rgb).not.toBeNull();

      const convertedHex = rgbToHex(rgb!.r, rgb!.g, rgb!.b);
      expect(convertedHex).toBe(originalHex);
    });

    it('should handle RGB to HSL to RGB conversion', () => {
      const originalRgb = { r: 255, g: 128, b: 64 };
      const hsl = rgbToHsl(originalRgb.r, originalRgb.g, originalRgb.b);
      const convertedRgb = hslToRgb(hsl.h, hsl.s, hsl.l);

      expect(convertedRgb.r).toBeCloseTo(originalRgb.r, 0);
      expect(convertedRgb.g).toBeCloseTo(originalRgb.g, 0);
      expect(convertedRgb.b).toBeCloseTo(originalRgb.b, 0);
    });
  });
});
