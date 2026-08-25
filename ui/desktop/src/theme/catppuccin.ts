import type { CatppuccinAccent, CatppuccinThemeId, ThemeVariant } from './types';
import { DEFAULT_CATPPUCCIN_ACCENT } from './types';

// Official Catppuccin palette: https://catppuccin.com/palette
export interface CatppuccinPalette {
  rosewater: string;
  flamingo: string;
  pink: string;
  mauve: string;
  red: string;
  maroon: string;
  peach: string;
  yellow: string;
  green: string;
  teal: string;
  sky: string;
  sapphire: string;
  blue: string;
  lavender: string;
  text: string;
  subtext1: string;
  subtext0: string;
  overlay2: string;
  overlay1: string;
  overlay0: string;
  surface2: string;
  surface1: string;
  surface0: string;
  base: string;
  mantle: string;
  crust: string;
}

export interface CatppuccinFlavorMeta {
  id: CatppuccinThemeId;
  name: string;
  variant: ThemeVariant;
  palette: CatppuccinPalette;
}

export const catppuccinFlavors: Record<CatppuccinThemeId, CatppuccinFlavorMeta> = {
  'catppuccin-latte': {
    id: 'catppuccin-latte',
    name: 'Latte',
    variant: 'light',
    palette: {
      rosewater: '#dc8a78',
      flamingo: '#dd7878',
      pink: '#ea76cb',
      mauve: '#8839ef',
      red: '#d20f39',
      maroon: '#e64553',
      peach: '#fe640b',
      yellow: '#df8e1d',
      green: '#40a02b',
      teal: '#179299',
      sky: '#04a5e5',
      sapphire: '#209fb5',
      blue: '#1e66f5',
      lavender: '#7287fd',
      text: '#4c4f69',
      subtext1: '#5c5f77',
      subtext0: '#6c6f85',
      overlay2: '#7c7f93',
      overlay1: '#8c8fa1',
      overlay0: '#9ca0b0',
      surface2: '#acb0be',
      surface1: '#bcc0cc',
      surface0: '#ccd0da',
      base: '#eff1f5',
      mantle: '#e6e9ef',
      crust: '#dce0e8',
    },
  },
  'catppuccin-frappe': {
    id: 'catppuccin-frappe',
    name: 'Frappé',
    variant: 'dark',
    palette: {
      rosewater: '#f2d5cf',
      flamingo: '#eebebe',
      pink: '#f4b8e4',
      mauve: '#ca9ee6',
      red: '#e78284',
      maroon: '#ea999c',
      peach: '#ef9f76',
      yellow: '#e5c890',
      green: '#a6d189',
      teal: '#81c8be',
      sky: '#99d1db',
      sapphire: '#85c1dc',
      blue: '#8caaee',
      lavender: '#babbf1',
      text: '#c6d0f5',
      subtext1: '#b5bfe2',
      subtext0: '#a5adce',
      overlay2: '#949cbb',
      overlay1: '#838ba7',
      overlay0: '#737994',
      surface2: '#626880',
      surface1: '#51576d',
      surface0: '#414559',
      base: '#303446',
      mantle: '#292c3c',
      crust: '#232634',
    },
  },
  'catppuccin-macchiato': {
    id: 'catppuccin-macchiato',
    name: 'Macchiato',
    variant: 'dark',
    palette: {
      rosewater: '#f4dbd6',
      flamingo: '#f0c6c6',
      pink: '#f5bde6',
      mauve: '#c6a0f6',
      red: '#ed8796',
      maroon: '#ee99a0',
      peach: '#f5a97f',
      yellow: '#eed49f',
      green: '#a6da95',
      teal: '#8bd5ca',
      sky: '#91d7e3',
      sapphire: '#7dc4e4',
      blue: '#8aadf4',
      lavender: '#b7bdf8',
      text: '#cad3f5',
      subtext1: '#b8c0e0',
      subtext0: '#a5adcb',
      overlay2: '#939ab7',
      overlay1: '#8087a2',
      overlay0: '#6e738d',
      surface2: '#5b6078',
      surface1: '#494d64',
      surface0: '#363a4f',
      base: '#24273a',
      mantle: '#1e2030',
      crust: '#181926',
    },
  },
  'catppuccin-mocha': {
    id: 'catppuccin-mocha',
    name: 'Mocha',
    variant: 'dark',
    palette: {
      rosewater: '#f5e0dc',
      flamingo: '#f2cdcd',
      pink: '#f5c2e7',
      mauve: '#cba6f7',
      red: '#f38ba8',
      maroon: '#eba0ac',
      peach: '#fab387',
      yellow: '#f9e2af',
      green: '#a6e3a1',
      teal: '#94e2d5',
      sky: '#89dceb',
      sapphire: '#74c7ec',
      blue: '#89b4fa',
      lavender: '#b4befe',
      text: '#cdd6f4',
      subtext1: '#bac2de',
      subtext0: '#a6adc8',
      overlay2: '#9399b2',
      overlay1: '#7f849c',
      overlay0: '#6c7086',
      surface2: '#585b70',
      surface1: '#45475a',
      surface0: '#313244',
      base: '#1e1e2e',
      mantle: '#181825',
      crust: '#11111b',
    },
  },
};

export interface CatppuccinPreview {
  background: string;
  surface: string;
  sidebar: string;
  text: string;
  muted: string;
  accent: string;
}

export function getCatppuccinPreview(
  flavorId: CatppuccinThemeId,
  accent: CatppuccinAccent = DEFAULT_CATPPUCCIN_ACCENT
): CatppuccinPreview {
  const { palette } = catppuccinFlavors[flavorId];
  return {
    background: palette.base,
    surface: palette.mantle,
    sidebar: palette.crust,
    text: palette.text,
    muted: palette.overlay0,
    accent: palette[accent],
  };
}

export function getCatppuccinAccentColor(
  flavorId: CatppuccinThemeId,
  accent: CatppuccinAccent
): string {
  return catppuccinFlavors[flavorId].palette[accent];
}

export interface CatppuccinColorTokens {
  '--color-background-primary': string;
  '--color-background-secondary': string;
  '--color-background-tertiary': string;
  '--color-background-inverse': string;
  '--color-background-ghost': string;
  '--color-background-info': string;
  '--color-background-danger': string;
  '--color-background-success': string;
  '--color-background-warning': string;
  '--color-background-disabled': string;
  '--color-text-primary': string;
  '--color-text-secondary': string;
  '--color-text-tertiary': string;
  '--color-text-inverse': string;
  '--color-text-ghost': string;
  '--color-text-info': string;
  '--color-text-danger': string;
  '--color-text-success': string;
  '--color-text-warning': string;
  '--color-text-disabled': string;
  '--color-border-primary': string;
  '--color-border-secondary': string;
  '--color-border-tertiary': string;
  '--color-border-inverse': string;
  '--color-border-ghost': string;
  '--color-border-info': string;
  '--color-border-danger': string;
  '--color-border-success': string;
  '--color-border-warning': string;
  '--color-border-disabled': string;
  '--color-ring-primary': string;
  '--color-ring-secondary': string;
  '--color-ring-inverse': string;
  '--color-ring-info': string;
  '--color-ring-danger': string;
  '--color-ring-success': string;
  '--color-ring-warning': string;
  '--shadow-hairline': string;
  '--shadow-sm': string;
  '--shadow-md': string;
  '--shadow-lg': string;
}

export function buildCatppuccinColorTokens(
  flavorId: CatppuccinThemeId,
  accent: CatppuccinAccent = DEFAULT_CATPPUCCIN_ACCENT
): CatppuccinColorTokens {
  const { palette, variant } = catppuccinFlavors[flavorId];
  const accentColor = palette[accent];
  const inverseText = variant === 'light' ? palette.base : palette.crust;
  const shadowRgb = variant === 'light' ? '76, 79, 105' : '17, 17, 27';

  return {
    '--color-background-primary': palette.base,
    '--color-background-secondary': palette.mantle,
    '--color-background-tertiary': palette.surface0,
    '--color-background-inverse': accentColor,
    '--color-background-ghost': 'transparent',
    '--color-background-info': accentColor,
    '--color-background-danger': palette.red,
    '--color-background-success': palette.green,
    '--color-background-warning': palette.yellow,
    '--color-background-disabled': palette.surface0,

    '--color-text-primary': palette.text,
    '--color-text-secondary': palette.subtext0,
    '--color-text-tertiary': palette.overlay1,
    '--color-text-inverse': inverseText,
    '--color-text-ghost': palette.subtext0,
    '--color-text-info': accentColor,
    '--color-text-danger': palette.red,
    '--color-text-success': palette.green,
    '--color-text-warning': palette.yellow,
    '--color-text-disabled': palette.overlay0,

    '--color-border-primary': palette.surface0,
    '--color-border-secondary': palette.surface1,
    '--color-border-tertiary': palette.surface2,
    '--color-border-inverse': palette.text,
    '--color-border-ghost': 'transparent',
    '--color-border-info': accentColor,
    '--color-border-danger': palette.red,
    '--color-border-success': palette.green,
    '--color-border-warning': palette.yellow,
    '--color-border-disabled': palette.surface0,

    '--color-ring-primary': palette.surface1,
    '--color-ring-secondary': palette.surface0,
    '--color-ring-inverse': palette.base,
    '--color-ring-info': accentColor,
    '--color-ring-danger': palette.red,
    '--color-ring-success': palette.green,
    '--color-ring-warning': palette.yellow,

    '--shadow-hairline': `0 0 0 1px rgba(${shadowRgb}, 0.12)`,
    '--shadow-sm': `0 1px 2px 0 rgba(${shadowRgb}, 0.16)`,
    '--shadow-md': `0 4px 6px -1px rgba(${shadowRgb}, 0.2), 0 2px 4px -2px rgba(${shadowRgb}, 0.14)`,
    '--shadow-lg': `0 10px 15px -3px rgba(${shadowRgb}, 0.22), 0 4px 6px -4px rgba(${shadowRgb}, 0.16)`,
  };
}
