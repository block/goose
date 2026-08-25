import { describe, expect, it } from 'vitest';
import {
  buildCatppuccinColorTokens,
  catppuccinFlavors,
  getCatppuccinAccentColor,
  getCatppuccinPreview,
} from './catppuccin';
import { CATPPUCCIN_ACCENTS, CATPPUCCIN_THEME_IDS, DEFAULT_CATPPUCCIN_ACCENT } from './types';
import { applyThemeTokens, getThemeTokens, themes } from './theme-tokens';

describe('Catppuccin palettes', () => {
  it('ships official hex values for all four flavors', () => {
    expect(catppuccinFlavors['catppuccin-latte'].palette.base).toBe('#eff1f5');
    expect(catppuccinFlavors['catppuccin-frappe'].palette.base).toBe('#303446');
    expect(catppuccinFlavors['catppuccin-macchiato'].palette.base).toBe('#24273a');
    expect(catppuccinFlavors['catppuccin-mocha'].palette.base).toBe('#1e1e2e');
    expect(catppuccinFlavors['catppuccin-latte'].variant).toBe('light');
    expect(catppuccinFlavors['catppuccin-mocha'].variant).toBe('dark');
  });

  it('maps the selected accent onto info and inverse tokens', () => {
    const tokens = buildCatppuccinColorTokens('catppuccin-mocha', 'teal');
    expect(tokens['--color-text-info']).toBe(
      getCatppuccinAccentColor('catppuccin-mocha', 'teal')
    );
    expect(tokens['--color-background-inverse']).toBe(
      getCatppuccinAccentColor('catppuccin-mocha', 'teal')
    );
    expect(tokens['--color-background-danger']).toBe(
      catppuccinFlavors['catppuccin-mocha'].palette.red
    );
    expect(tokens['--color-background-success']).toBe(
      catppuccinFlavors['catppuccin-mocha'].palette.green
    );
  });

  it('defaults the accent to mauve', () => {
    const preview = getCatppuccinPreview('catppuccin-mocha');
    expect(preview.accent).toBe(
      getCatppuccinAccentColor('catppuccin-mocha', DEFAULT_CATPPUCCIN_ACCENT)
    );
    expect(CATPPUCCIN_ACCENTS).toContain(DEFAULT_CATPPUCCIN_ACCENT);
  });

  it('registers each flavor in the theme map', () => {
    for (const flavorId of CATPPUCCIN_THEME_IDS) {
      expect(themes[flavorId].variant).toBe(catppuccinFlavors[flavorId].variant);
      const tokens = getThemeTokens(flavorId, 'pink');
      expect(tokens['--color-background-primary']).toBe(
        catppuccinFlavors[flavorId].palette.base
      );
      expect(tokens['--color-text-info']).toBe(
        getCatppuccinAccentColor(flavorId, 'pink')
      );
    }
  });

  it('applies flavor tokens to the document root', () => {
    applyThemeTokens('catppuccin-mocha', 'blue');
    expect(document.documentElement.style.getPropertyValue('--color-background-primary')).toBe(
      '#1e1e2e'
    );
    expect(document.documentElement.style.getPropertyValue('--color-text-info')).toBe(
      getCatppuccinAccentColor('catppuccin-mocha', 'blue')
    );
  });
});
