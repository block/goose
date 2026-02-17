/**
 * High Contrast Theme
 * Maximum contrast for accessibility
 */

import { ThemePreset } from './types';

export const highContrast: ThemePreset = {
  id: 'high-contrast',
  name: 'High Contrast',
  author: 'Block',
  description: 'Maximum contrast theme optimized for accessibility and readability',
  tags: ['light', 'dark', 'high-contrast', 'accessible'],
  version: '1.0.0',
  colors: {
    light: {
      'color-background-primary': '#ffffff',
      'color-background-secondary': '#f0f0f0',
      'color-background-tertiary': '#e0e0e0',
      'color-background-inverse': '#000000',
      'color-background-danger': '#d32f2f',
      'color-background-info': '#1976d2',
      
      'color-border-primary': '#000000',
      'color-border-secondary': '#000000',
      'color-border-danger': '#d32f2f',
      'color-border-info': '#1976d2',
      
      'color-text-primary': '#000000',
      'color-text-secondary': '#424242',
      'color-text-inverse': '#ffffff',
      'color-text-danger': '#d32f2f',
      'color-text-success': '#2e7d32',
      'color-text-warning': '#f57c00',
      'color-text-info': '#1976d2',
      
      'color-ring-primary': '#000000',
    },
    dark: {
      'color-background-primary': '#000000',
      'color-background-secondary': '#1a1a1a',
      'color-background-tertiary': '#2a2a2a',
      'color-background-inverse': '#ffffff',
      'color-background-danger': '#ff5252',
      'color-background-info': '#448aff',
      
      'color-border-primary': '#ffffff',
      'color-border-secondary': '#ffffff',
      'color-border-danger': '#ff5252',
      'color-border-info': '#448aff',
      
      'color-text-primary': '#ffffff',
      'color-text-secondary': '#e0e0e0',
      'color-text-inverse': '#000000',
      'color-text-danger': '#ff5252',
      'color-text-success': '#69f0ae',
      'color-text-warning': '#ffab40',
      'color-text-info': '#448aff',
      
      'color-ring-primary': '#ffffff',
    },
  },
};
