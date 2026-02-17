/**
 * Solarized Theme
 * Precision colors for machines and people
 */

import { ThemePreset } from './types';

export const solarized: ThemePreset = {
  id: 'solarized',
  name: 'Solarized',
  author: 'Ethan Schoonover',
  description: 'Precision colors for machines and people - designed for optimal readability',
  tags: ['light', 'dark', 'minimal', 'retro'],
  version: '1.0.0',
  colors: {
    light: {
      'color-background-primary': '#fdf6e3',
      'color-background-secondary': '#eee8d5',
      'color-background-tertiary': '#e3dcc3',
      'color-background-inverse': '#002b36',
      'color-background-danger': '#dc322f',
      'color-background-info': '#268bd2',
      
      'color-border-primary': '#e3dcc3',
      'color-border-secondary': '#d3cdb3',
      'color-border-danger': '#dc322f',
      'color-border-info': '#268bd2',
      
      'color-text-primary': '#657b83',
      'color-text-secondary': '#93a1a1',
      'color-text-inverse': '#fdf6e3',
      'color-text-danger': '#dc322f',
      'color-text-success': '#859900',
      'color-text-warning': '#b58900',
      'color-text-info': '#268bd2',
      
      'color-ring-primary': '#d3cdb3',
    },
    dark: {
      'color-background-primary': '#002b36',
      'color-background-secondary': '#073642',
      'color-background-tertiary': '#0d4654',
      'color-background-inverse': '#fdf6e3',
      'color-background-danger': '#dc322f',
      'color-background-info': '#268bd2',
      
      'color-border-primary': '#073642',
      'color-border-secondary': '#586e75',
      'color-border-danger': '#dc322f',
      'color-border-info': '#268bd2',
      
      'color-text-primary': '#839496',
      'color-text-secondary': '#657b83',
      'color-text-inverse': '#002b36',
      'color-text-danger': '#dc322f',
      'color-text-success': '#859900',
      'color-text-warning': '#b58900',
      'color-text-info': '#268bd2',
      
      'color-ring-primary': '#586e75',
    },
  },
};
