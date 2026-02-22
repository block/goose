/**
 * GitHub Theme
 * Clean and familiar GitHub colors
 */

import { ThemePreset } from './types';

export const github: ThemePreset = {
  id: 'github',
  name: 'GitHub',
  author: 'GitHub',
  description: 'Clean, familiar colors from GitHub - professional and easy on the eyes',
  tags: ['light', 'dark', 'minimal', 'modern'],
  version: '1.0.0',
  colors: {
    light: {
      'color-background-primary': '#ffffff',
      'color-background-secondary': '#f6f8fa',
      'color-background-tertiary': '#eaeef2',
      'color-background-inverse': '#24292f',
      'color-background-danger': '#d1242f',
      'color-background-info': '#0969da',
      
      'color-border-primary': '#d0d7de',
      'color-border-secondary': '#d0d7de',
      'color-border-danger': '#d1242f',
      'color-border-info': '#0969da',
      
      'color-text-primary': '#24292f',
      'color-text-secondary': '#57606a',
      'color-text-inverse': '#ffffff',
      'color-text-danger': '#d1242f',
      'color-text-success': '#1a7f37',
      'color-text-warning': '#9a6700',
      'color-text-info': '#0969da',
      
      'color-ring-primary': '#d0d7de',
    },
    dark: {
      'color-background-primary': '#0d1117',
      'color-background-secondary': '#161b22',
      'color-background-tertiary': '#21262d',
      'color-background-inverse': '#f0f6fc',
      'color-background-danger': '#da3633',
      'color-background-info': '#58a6ff',
      
      'color-border-primary': '#30363d',
      'color-border-secondary': '#484f58',
      'color-border-danger': '#da3633',
      'color-border-info': '#58a6ff',
      
      'color-text-primary': '#e6edf3',
      'color-text-secondary': '#7d8590',
      'color-text-inverse': '#0d1117',
      'color-text-danger': '#ff7b72',
      'color-text-success': '#3fb950',
      'color-text-warning': '#d29922',
      'color-text-info': '#79c0ff',
      
      'color-ring-primary': '#484f58',
    },
  },
};
