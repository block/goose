/**
 * Gruvbox Theme
 * Warm, retro-inspired color palette
 */

import { ThemePreset } from './types';

export const gruvbox: ThemePreset = {
  id: 'gruvbox',
  name: 'Gruvbox',
  author: 'Pavel Pertsev',
  description: 'Warm, retro groove colors designed for long coding sessions',
  tags: ['dark', 'light', 'warm', 'retro'],
  version: '1.0.0',
  colors: {
    light: {
      'color-background-primary': '#fbf1c7',
      'color-background-secondary': '#f2e5bc',
      'color-background-tertiary': '#ebdbb2',
      'color-background-inverse': '#282828',
      'color-background-danger': '#cc241d',
      'color-background-info': '#458588',
      
      'color-border-primary': '#ebdbb2',
      'color-border-secondary': '#d5c4a1',
      'color-border-danger': '#cc241d',
      'color-border-info': '#458588',
      
      'color-text-primary': '#3c3836',
      'color-text-secondary': '#7c6f64',
      'color-text-inverse': '#fbf1c7',
      'color-text-danger': '#cc241d',
      'color-text-success': '#98971a',
      'color-text-warning': '#d79921',
      'color-text-info': '#458588',
      
      'color-ring-primary': '#d5c4a1',
    },
    dark: {
      'color-background-primary': '#282828',
      'color-background-secondary': '#3c3836',
      'color-background-tertiary': '#504945',
      'color-background-inverse': '#fbf1c7',
      'color-background-danger': '#fb4934',
      'color-background-info': '#83a598',
      
      'color-border-primary': '#3c3836',
      'color-border-secondary': '#665c54',
      'color-border-danger': '#fb4934',
      'color-border-info': '#83a598',
      
      'color-text-primary': '#ebdbb2',
      'color-text-secondary': '#a89984',
      'color-text-inverse': '#282828',
      'color-text-danger': '#fb4934',
      'color-text-success': '#b8bb26',
      'color-text-warning': '#fabd2f',
      'color-text-info': '#83a598',
      
      'color-ring-primary': '#665c54',
    },
  },
};
