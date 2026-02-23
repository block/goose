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
    description: 'Chat area, main content, message list',
  },
  {
    name: 'color-background-secondary',
    label: 'Secondary Background',
    category: 'background',
    description: 'Sidebar, cards, settings panels',
  },
  {
    name: 'color-background-tertiary',
    label: 'Tertiary Background',
    category: 'background',
    description: 'Hover states, nested panels, active items',
  },
  {
    name: 'color-background-inverse',
    label: 'Inverse Background',
    category: 'background',
    description: 'Primary buttons, selected states',
  },
  {
    name: 'color-background-danger',
    label: 'Danger Background',
    category: 'background',
    description: 'Error messages, delete buttons',
  },
  {
    name: 'color-background-info',
    label: 'Info Background',
    category: 'background',
    description: 'Info messages, tips, help text',
  },
  
  // Border colors
  {
    name: 'color-border-primary',
    label: 'Primary Border',
    category: 'border',
    description: 'Cards, inputs, dividers, separators',
  },
  {
    name: 'color-border-secondary',
    label: 'Secondary Border',
    category: 'border',
    description: 'Hover states, focus states',
  },
  {
    name: 'color-border-danger',
    label: 'Danger Border',
    category: 'border',
    description: 'Error inputs, warning boxes',
  },
  {
    name: 'color-border-info',
    label: 'Info Border',
    category: 'border',
    description: 'Info boxes, help panels',
  },
  
  // Text colors
  {
    name: 'color-text-primary',
    label: 'Primary Text',
    category: 'text',
    description: 'Headings, body text, chat messages',
  },
  {
    name: 'color-text-secondary',
    label: 'Secondary Text',
    category: 'text',
    description: 'Labels, captions, timestamps, metadata',
  },
  {
    name: 'color-text-inverse',
    label: 'Inverse Text',
    category: 'text',
    description: 'Text on buttons, selected items',
  },
  {
    name: 'color-text-danger',
    label: 'Danger Text',
    category: 'text',
    description: 'Error messages, delete actions',
  },
  {
    name: 'color-text-success',
    label: 'Success Text',
    category: 'text',
    description: 'Success messages, confirmations',
  },
  {
    name: 'color-text-warning',
    label: 'Warning Text',
    category: 'text',
    description: 'Warning messages, caution text',
  },
  {
    name: 'color-text-info',
    label: 'Info Text',
    category: 'text',
    description: 'Info messages, tips, help text',
  },
  
  // Ring colors
  {
    name: 'color-ring-primary',
    label: 'Primary Ring',
    category: 'ring',
    description: 'Focus rings on buttons, inputs (accessibility)',
  },
];
