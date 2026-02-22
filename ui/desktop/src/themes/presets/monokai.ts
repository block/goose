/**
 * Monokai Theme
 * Developer favorite from Sublime Text
 */

import { ThemePreset } from './types';

export const monokai: ThemePreset = {
  id: 'monokai',
  name: 'Monokai',
  author: 'Wimer Hazenberg',
  description: 'Classic developer theme from Sublime Text with vibrant syntax colors',
  tags: ['dark', 'colorful', 'retro'],
  version: '1.0.0',
  colors: {
    light: {
      'color-background-primary': '#fafafa',
      'color-background-secondary': '#f5f5f5',
      'color-background-tertiary': '#e8e8e8',
      'color-background-inverse': '#272822',
      'color-background-danger': '#f92672',
      'color-background-info': '#66d9ef',
      
      'color-border-primary': '#e8e8e8',
      'color-border-secondary': '#d8d8d8',
      'color-border-danger': '#f92672',
      'color-border-info': '#66d9ef',
      
      'color-text-primary': '#272822',
      'color-text-secondary': '#75715e',
      'color-text-inverse': '#f8f8f2',
      'color-text-danger': '#f92672',
      'color-text-success': '#a6e22e',
      'color-text-warning': '#e6db74',
      'color-text-info': '#66d9ef',
      
      'color-ring-primary': '#d8d8d8',
    },
    dark: {
      'color-background-primary': '#272822',
      'color-background-secondary': '#3e3d32',
      'color-background-tertiary': '#49483e',
      'color-background-inverse': '#f8f8f2',
      'color-background-danger': '#f92672',
      'color-background-info': '#66d9ef',
      
      'color-border-primary': '#3e3d32',
      'color-border-secondary': '#75715e',
      'color-border-danger': '#f92672',
      'color-border-info': '#66d9ef',
      
      'color-text-primary': '#f8f8f2',
      'color-text-secondary': '#75715e',
      'color-text-inverse': '#272822',
      'color-text-danger': '#f92672',
      'color-text-success': '#a6e22e',
      'color-text-warning': '#e6db74',
      'color-text-info': '#66d9ef',
      
      'color-ring-primary': '#75715e',
    },
  },
};
