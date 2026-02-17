/**
 * Nord Theme
 * Arctic, north-bluish color palette
 */

import { ThemePreset } from './types';

export const nord: ThemePreset = {
  id: 'nord',
  name: 'Nord',
  author: 'Arctic Ice Studio',
  description: 'An arctic, north-bluish color palette with clean and elegant design',
  tags: ['dark', 'light', 'cool', 'minimal'],
  version: '1.0.0',
  colors: {
    light: {
      'color-background-primary': '#eceff4',
      'color-background-secondary': '#e5e9f0',
      'color-background-tertiary': '#d8dee9',
      'color-background-inverse': '#2e3440',
      'color-background-danger': '#bf616a',
      'color-background-info': '#5e81ac',
      
      'color-border-primary': '#d8dee9',
      'color-border-secondary': '#d8dee9',
      'color-border-danger': '#bf616a',
      'color-border-info': '#5e81ac',
      
      'color-text-primary': '#2e3440',
      'color-text-secondary': '#4c566a',
      'color-text-inverse': '#eceff4',
      'color-text-danger': '#bf616a',
      'color-text-success': '#a3be8c',
      'color-text-warning': '#ebcb8b',
      'color-text-info': '#5e81ac',
      
      'color-ring-primary': '#d8dee9',
    },
    dark: {
      'color-background-primary': '#2e3440',
      'color-background-secondary': '#3b4252',
      'color-background-tertiary': '#434c5e',
      'color-background-inverse': '#eceff4',
      'color-background-danger': '#bf616a',
      'color-background-info': '#81a1c1',
      
      'color-border-primary': '#3b4252',
      'color-border-secondary': '#4c566a',
      'color-border-danger': '#bf616a',
      'color-border-info': '#81a1c1',
      
      'color-text-primary': '#eceff4',
      'color-text-secondary': '#d8dee9',
      'color-text-inverse': '#2e3440',
      'color-text-danger': '#bf616a',
      'color-text-success': '#a3be8c',
      'color-text-warning': '#ebcb8b',
      'color-text-info': '#88c0d0',
      
      'color-ring-primary': '#4c566a',
    },
  },
};
