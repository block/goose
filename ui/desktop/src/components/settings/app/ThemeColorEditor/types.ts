/**
 * ThemeColorEditor Types
 */

export interface ThemeColors {
  light: Record<string, string>;
  dark: Record<string, string>;
}

export interface ThemeColorEditorProps {
  onClose: () => void;
}

export type ColorMode = 'light' | 'dark';

export interface ColorVariable {
  name: string;
  label: string;
  category: 'background' | 'border' | 'text' | 'ring';
  description?: string;
}

export const COLOR_VARIABLES: ColorVariable[] = [
  // Background colors
  {
    name: 'color-background-primary',
    label: 'Primary Background',
    category: 'background',
    description: 'Main background color for the app',
  },
  {
    name: 'color-background-secondary',
    label: 'Secondary Background',
    category: 'background',
    description: 'Secondary background for cards and panels',
  },
  {
    name: 'color-background-tertiary',
    label: 'Tertiary Background',
    category: 'background',
    description: 'Tertiary background for nested elements',
  },
  {
    name: 'color-background-inverse',
    label: 'Inverse Background',
    category: 'background',
    description: 'Inverse background for high contrast elements',
  },
  {
    name: 'color-background-danger',
    label: 'Danger Background',
    category: 'background',
    description: 'Background for error states',
  },
  {
    name: 'color-background-info',
    label: 'Info Background',
    category: 'background',
    description: 'Background for informational elements',
  },
  
  // Border colors
  {
    name: 'color-border-primary',
    label: 'Primary Border',
    category: 'border',
    description: 'Main border color',
  },
  {
    name: 'color-border-secondary',
    label: 'Secondary Border',
    category: 'border',
    description: 'Secondary border color',
  },
  {
    name: 'color-border-danger',
    label: 'Danger Border',
    category: 'border',
    description: 'Border for error states',
  },
  {
    name: 'color-border-info',
    label: 'Info Border',
    category: 'border',
    description: 'Border for informational elements',
  },
  
  // Text colors
  {
    name: 'color-text-primary',
    label: 'Primary Text',
    category: 'text',
    description: 'Main text color',
  },
  {
    name: 'color-text-secondary',
    label: 'Secondary Text',
    category: 'text',
    description: 'Secondary text color for less emphasis',
  },
  {
    name: 'color-text-inverse',
    label: 'Inverse Text',
    category: 'text',
    description: 'Text color on dark backgrounds',
  },
  {
    name: 'color-text-danger',
    label: 'Danger Text',
    category: 'text',
    description: 'Text color for errors',
  },
  {
    name: 'color-text-success',
    label: 'Success Text',
    category: 'text',
    description: 'Text color for success states',
  },
  {
    name: 'color-text-warning',
    label: 'Warning Text',
    category: 'text',
    description: 'Text color for warnings',
  },
  {
    name: 'color-text-info',
    label: 'Info Text',
    category: 'text',
    description: 'Text color for informational content',
  },
  
  // Ring colors
  {
    name: 'color-ring-primary',
    label: 'Primary Ring',
    category: 'ring',
    description: 'Focus ring color',
  },
];
