export interface CustomCommand {
  id: string;
  name: string; // The command name (e.g., "document", "review")
  label: string; // Display name (e.g., "Create Document", "Code Review")
  description: string; // Short description for the popover
  prompt: string; // The full prompt template that gets sent to the LLM
  icon?: string; // Optional icon name or emoji
  category?: string; // Optional category for grouping
  variables?: CustomCommandVariable[]; // Template variables
  createdAt: Date;
  updatedAt: Date;
  usageCount: number; // Track how often it's used
  isFavorite: boolean; // User can mark favorites
}

export interface CustomCommandVariable {
  name: string; // Variable name (e.g., "filename", "selection")
  label: string; // Display label
  description: string; // Help text
  type: 'text' | 'selection' | 'filename' | 'directory' | 'custom';
  required: boolean;
  defaultValue?: string;
}

export interface CustomCommandCategory {
  id: string;
  name: string;
  description: string;
  color: string; // Hex color for visual grouping
  icon?: string;
}

export interface CustomCommandsState {
  commands: CustomCommand[];
  categories: CustomCommandCategory[];
  isLoading: boolean;
  error: string | null;
}

// Built-in command categories
export const DEFAULT_CATEGORIES: CustomCommandCategory[] = [
  {
    id: 'general',
    name: 'General',
    description: 'General purpose commands',
    color: '#6B7280',
    icon: 'Zap'
  },
  {
    id: 'development',
    name: 'Development',
    description: 'Code and development related commands',
    color: '#3B82F6',
    icon: 'Code'
  },
  {
    id: 'documentation',
    name: 'Documentation',
    description: 'Documentation and writing commands',
    color: '#10B981',
    icon: 'FileText'
  },
  {
    id: 'analysis',
    name: 'Analysis',
    description: 'Analysis and review commands',
    color: '#8B5CF6',
    icon: 'Search'
  }
];

// Template for creating new commands
export const COMMAND_TEMPLATE: Omit<CustomCommand, 'id' | 'createdAt' | 'updatedAt'> = {
  name: '',
  label: '',
  description: '',
  prompt: '',
  icon: 'Zap',
  category: 'general',
  variables: [],
  usageCount: 0,
  isFavorite: false
};

// Validation rules
export const COMMAND_VALIDATION = {
  name: {
    minLength: 2,
    maxLength: 20,
    pattern: /^[a-z][a-z0-9]*$/i, // Must start with letter, only alphanumeric
  },
  label: {
    minLength: 3,
    maxLength: 50,
  },
  description: {
    minLength: 10,
    maxLength: 200,
  },
  prompt: {
    minLength: 10,
    maxLength: 5000,
  }
};

// Common variable templates
export const COMMON_VARIABLES: CustomCommandVariable[] = [
  {
    name: 'selection',
    label: 'Selected Text',
    description: 'Currently selected text in the editor',
    type: 'selection',
    required: false,
  },
  {
    name: 'filename',
    label: 'Current File',
    description: 'Name of the currently active file',
    type: 'filename',
    required: false,
  },
  {
    name: 'directory',
    label: 'Current Directory',
    description: 'Current working directory path',
    type: 'directory',
    required: false,
  }
];
