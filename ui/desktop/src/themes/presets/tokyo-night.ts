/**
 * Tokyo Night Theme
 * Modern, vibrant night theme
 */

import { ThemePreset } from './types';

export const tokyoNight: ThemePreset = {
  id: 'tokyo-night',
  name: 'Tokyo Night',
  author: 'Folke Lemaitre',
  description: 'A clean, dark theme inspired by the lights of Tokyo at night',
  tags: ['dark', 'modern', 'colorful'],
  version: '1.0.0',
  colors: {
    light: {
      'color-background-primary': '#d5d6db',
      'color-background-secondary': '#cbccd1',
      'color-background-tertiary': '#c4c8da',
      'color-background-inverse': '#1a1b26',
      'color-background-danger': '#f52a65',
      'color-background-info': '#2ac3de',
      
      'color-border-primary': '#c4c8da',
      'color-border-secondary': '#a8aecb',
      'color-border-danger': '#f52a65',
      'color-border-info': '#2ac3de',
      
      'color-text-primary': '#343b58',
      'color-text-secondary': '#565a6e',
      'color-text-inverse': '#d5d6db',
      'color-text-danger': '#f52a65',
      'color-text-success': '#33635c',
      'color-text-warning': '#8c6c3e',
      'color-text-info': '#2e7de9',
      
      'color-ring-primary': '#a8aecb',
    },
    dark: {
      'color-background-primary': '#1a1b26',
      'color-background-secondary': '#24283b',
      'color-background-tertiary': '#414868',
      'color-background-inverse': '#c0caf5',
      'color-background-danger': '#f7768e',
      'color-background-info': '#7dcfff',
      
      'color-border-primary': '#24283b',
      'color-border-secondary': '#414868',
      'color-border-danger': '#f7768e',
      'color-border-info': '#7dcfff',
      
      'color-text-primary': '#c0caf5',
      'color-text-secondary': '#565f89',
      'color-text-inverse': '#1a1b26',
      'color-text-danger': '#f7768e',
      'color-text-success': '#9ece6a',
      'color-text-warning': '#e0af68',
      'color-text-info': '#7aa2f7',
      
      'color-ring-primary': '#414868',
    },
  },
};
