/**
 * Goose Classic Theme
 * The default Goose Desktop theme
 */

import { ThemePreset } from './types';

export const gooseClassic: ThemePreset = {
  id: 'goose-classic',
  name: 'Goose Classic',
  author: 'Block',
  description: 'The default Goose Desktop theme with clean, professional colors',
  tags: ['light', 'dark', 'default'],
  version: '1.0.0',
  colors: {
    light: {
      'color-background-primary': '#ffffff',
      'color-background-secondary': '#f4f6f7',
      'color-background-tertiary': '#e3e6ea',
      'color-background-inverse': '#000000',
      'color-background-danger': '#f94b4b',
      'color-background-info': '#5c98f9',
      
      'color-border-primary': '#e3e6ea',
      'color-border-secondary': '#e3e6ea',
      'color-border-danger': '#f94b4b',
      'color-border-info': '#5c98f9',
      
      'color-text-primary': '#3f434b',
      'color-text-secondary': '#878787',
      'color-text-inverse': '#ffffff',
      'color-text-danger': '#f94b4b',
      'color-text-success': '#91cb80',
      'color-text-warning': '#fbcd44',
      'color-text-info': '#5c98f9',
      
      'color-ring-primary': '#e3e6ea',
    },
    dark: {
      'color-background-primary': '#22252a',
      'color-background-secondary': '#3f434b',
      'color-background-tertiary': '#474e57',
      'color-background-inverse': '#cbd1d6',
      'color-background-danger': '#ff6b6b',
      'color-background-info': '#7cacff',
      
      'color-border-primary': '#3f434b',
      'color-border-secondary': '#606c7a',
      'color-border-danger': '#ff6b6b',
      'color-border-info': '#7cacff',
      
      'color-text-primary': '#ffffff',
      'color-text-secondary': '#878787',
      'color-text-inverse': '#000000',
      'color-text-danger': '#ff6b6b',
      'color-text-success': '#a3d795',
      'color-text-warning': '#ffd966',
      'color-text-info': '#7cacff',
      
      'color-ring-primary': '#606c7a',
    },
  },
};
