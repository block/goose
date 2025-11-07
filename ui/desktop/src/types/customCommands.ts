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
  isBuiltIn?: boolean; // Built-in commands cannot be deleted, only favorited
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

// Built-in commands that cannot be deleted, only favorited
export const BUILT_IN_COMMANDS: CustomCommand[] = [
  {
    id: 'builtin_explain',
    name: 'explain',
    label: 'Explain Code',
    description: 'Provide detailed explanation of code or concepts',
    prompt: `Please explain the provided code or concept in detail. Include:

1. What it does (high-level purpose)
2. How it works (step-by-step breakdown)
3. Key concepts and patterns used
4. Dependencies and requirements
5. Potential use cases
6. Any notable design decisions

Make the explanation accessible and comprehensive.`,
    icon: 'Search',
    category: 'development',
    variables: [],
    createdAt: new Date('2024-01-01'),
    updatedAt: new Date('2024-01-01'),
    usageCount: 0,
    isFavorite: false,
    isBuiltIn: true,
  },
  {
    id: 'builtin_review',
    name: 'review',
    label: 'Code Review',
    description: 'Perform thorough code review with suggestions',
    prompt: `Please perform a comprehensive code review. Focus on:

1. Code quality and readability
2. Performance optimizations
3. Security considerations
4. Best practices and conventions
5. Potential bugs or issues
6. Suggestions for improvement

Provide specific, actionable feedback with examples where appropriate.`,
    icon: 'Search',
    category: 'development',
    variables: [],
    createdAt: new Date('2024-01-01'),
    updatedAt: new Date('2024-01-01'),
    usageCount: 0,
    isFavorite: false,
    isBuiltIn: true,
  },
  {
    id: 'builtin_document',
    name: 'document',
    label: 'Create Documentation',
    description: 'Generate comprehensive documentation',
    prompt: `Please create comprehensive documentation for the provided code or project. Include:

1. Overview and purpose
2. Installation/setup instructions
3. Usage examples
4. API reference (if applicable)
5. Configuration options
6. Troubleshooting guide

Make the documentation clear, well-structured, and suitable for both beginners and experienced users.`,
    icon: 'FileText',
    category: 'documentation',
    variables: [],
    createdAt: new Date('2024-01-01'),
    updatedAt: new Date('2024-01-01'),
    usageCount: 0,
    isFavorite: false,
    isBuiltIn: true,
  },
  {
    id: 'builtin_optimize',
    name: 'optimize',
    label: 'Optimize Code',
    description: 'Suggest optimizations for better performance',
    prompt: `Please analyze the provided code and suggest optimizations for better performance, efficiency, and maintainability. Consider:

1. Algorithm efficiency and time complexity
2. Memory usage optimization
3. Code structure and organization
4. Best practices for the specific language/framework
5. Potential refactoring opportunities
6. Performance bottlenecks

Provide specific, actionable recommendations with code examples where helpful.`,
    icon: 'Zap',
    category: 'development',
    variables: [],
    createdAt: new Date('2024-01-01'),
    updatedAt: new Date('2024-01-01'),
    usageCount: 0,
    isFavorite: false,
    isBuiltIn: true,
  },
  {
    id: 'builtin_test',
    name: 'test',
    label: 'Generate Tests',
    description: 'Create comprehensive unit tests',
    prompt: `Please generate comprehensive unit tests for the provided code. Include:

1. Test cases for normal/expected behavior
2. Edge cases and boundary conditions
3. Error handling and exception cases
4. Mock objects where appropriate
5. Test setup and teardown if needed
6. Clear, descriptive test names

Use the appropriate testing framework for the language and follow testing best practices.`,
    icon: 'Code',
    category: 'development',
    variables: [],
    createdAt: new Date('2024-01-01'),
    updatedAt: new Date('2024-01-01'),
    usageCount: 0,
    isFavorite: false,
    isBuiltIn: true,
  },
  {
    id: 'builtin_summarize',
    name: 'summarize',
    label: 'Summarize Content',
    description: 'Create concise summary of content',
    prompt: `Please create a concise summary of the provided content. Include:

1. Main points and key takeaways
2. Important details and findings
3. Conclusions or recommendations
4. Action items (if applicable)

Keep the summary clear, well-organized, and focused on the most important information.`,
    icon: 'FileText',
    category: 'general',
    variables: [],
    createdAt: new Date('2024-01-01'),
    updatedAt: new Date('2024-01-01'),
    usageCount: 0,
    isFavorite: false,
    isBuiltIn: true,
  },
];
