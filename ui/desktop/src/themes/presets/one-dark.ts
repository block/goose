/**
 * One Dark Theme
 * Atom editor inspired dark theme
 */

import { ThemePreset } from './types';

export const oneDark: ThemePreset = {
  id: 'one-dark',
  name: 'One Dark',
  author: 'Atom',
  description: 'Popular dark theme from Atom editor with balanced colors',
  tags: ['dark', 'modern', 'minimal'],
  version: '1.0.0',
  colors: {
    light: {
      'color-background-primary': '#fafafa',
      'color-background-secondary': '#f0f0f0',
      'color-background-tertiary': '#e5e5e5',
      'color-background-inverse': '#282c34',
      'color-background-danger': '#e45649',
      'color-background-info': '#4078f2',
      
      'color-border-primary': '#e5e5e5',
      'color-border-secondary': '#d0d0d0',
      'color-border-danger': '#e45649',
      'color-border-info': '#4078f2',
      
      'color-text-primary': '#383a42',
      'color-text-secondary': '#a0a1a7',
      'color-text-inverse': '#fafafa',
      'color-text-danger': '#e45649',
      'color-text-success': '#50a14f',
      'color-text-warning': '#c18401',
      'color-text-info': '#4078f2',
      
      'color-ring-primary': '#d0d0d0',
    },
    dark: {
      'color-background-primary': '#282c34',
      'color-background-secondary': '#21252b',
      'color-background-tertiary': '#2c313c',
      'color-background-inverse': '#abb2bf',
      'color-background-danger': '#e06c75',
      'color-background-info': '#61afef',
      
      'color-border-primary': '#21252b',
      'color-border-secondary': '#3e4451',
      'color-border-danger': '#e06c75',
      'color-border-info': '#61afef',
      
      'color-text-primary': '#abb2bf',
      'color-text-secondary': '#5c6370',
      'color-text-inverse': '#282c34',
      'color-text-danger': '#e06c75',
      'color-text-success': '#98c379',
      'color-text-warning': '#e5c07b',
      'color-text-info': '#61afef',
      
      'color-ring-primary': '#3e4451',
    },
  },
};
