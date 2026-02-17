/**
 * Dracula Theme
 * A dark theme with vibrant colors
 */

import { ThemePreset } from './types';

export const dracula: ThemePreset = {
  id: 'dracula',
  name: 'Dracula',
  author: 'Dracula Theme',
  description: 'A dark theme with vibrant, high-contrast colors perfect for long coding sessions',
  tags: ['dark', 'colorful', 'high-contrast'],
  version: '1.0.0',
  colors: {
    light: {
      'color-background-primary': '#f8f8f2',
      'color-background-secondary': '#f0f0eb',
      'color-background-tertiary': '#e6e6e1',
      'color-background-inverse': '#282a36',
      'color-background-danger': '#ff5555',
      'color-background-info': '#8be9fd',
      
      'color-border-primary': '#e6e6e1',
      'color-border-secondary': '#e6e6e1',
      'color-border-danger': '#ff5555',
      'color-border-info': '#8be9fd',
      
      'color-text-primary': '#282a36',
      'color-text-secondary': '#6272a4',
      'color-text-inverse': '#f8f8f2',
      'color-text-danger': '#ff5555',
      'color-text-success': '#50fa7b',
      'color-text-warning': '#f1fa8c',
      'color-text-info': '#8be9fd',
      
      'color-ring-primary': '#e6e6e1',
    },
    dark: {
      'color-background-primary': '#282a36',
      'color-background-secondary': '#343746',
      'color-background-tertiary': '#44475a',
      'color-background-inverse': '#f8f8f2',
      'color-background-danger': '#ff5555',
      'color-background-info': '#8be9fd',
      
      'color-border-primary': '#44475a',
      'color-border-secondary': '#6272a4',
      'color-border-danger': '#ff5555',
      'color-border-info': '#8be9fd',
      
      'color-text-primary': '#f8f8f2',
      'color-text-secondary': '#f8f8f2',
      'color-text-inverse': '#282a36',
      'color-text-danger': '#ff5555',
      'color-text-success': '#50fa7b',
      'color-text-warning': '#f1fa8c',
      'color-text-info': '#8be9fd',
      
      'color-ring-primary': '#6272a4',
    },
  },
};
