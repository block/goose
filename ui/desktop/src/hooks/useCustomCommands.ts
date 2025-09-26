import { useState, useEffect, useCallback } from 'react';
import { CustomCommand } from '../types/customCommands';

export const useCustomCommands = () => {
  const [commands, setCommands] = useState<CustomCommand[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Load commands from storage
  const loadCommands = useCallback(async () => {
    try {
      setIsLoading(true);
      setError(null);

      // For now, use localStorage. In the future, this could be replaced with API calls
      const stored = localStorage.getItem('goose-custom-commands');
      if (stored) {
        const parsed = JSON.parse(stored);
        const commands = parsed.map((cmd: any) => ({
          ...cmd,
          createdAt: new Date(cmd.createdAt),
          updatedAt: new Date(cmd.updatedAt)
        }));
        setCommands(commands);
      } else {
        // Load example commands for demo
        const exampleCommands = getExampleCommands();
        setCommands(exampleCommands);
        localStorage.setItem('goose-custom-commands', JSON.stringify(exampleCommands));
      }
    } catch (err) {
      console.error('Failed to load custom commands:', err);
      setError('Failed to load custom commands');
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Save commands to storage
  const saveCommands = useCallback(async (updatedCommands: CustomCommand[]) => {
    try {
      localStorage.setItem('goose-custom-commands', JSON.stringify(updatedCommands));
      setCommands(updatedCommands);
      setError(null);
    } catch (err) {
      console.error('Failed to save custom commands:', err);
      setError('Failed to save custom commands');
    }
  }, []);

  // Get command by ID
  const getCommand = useCallback((id: string): CustomCommand | undefined => {
    return commands.find(cmd => cmd.id === id);
  }, [commands]);

  // Get command by name
  const getCommandByName = useCallback((name: string): CustomCommand | undefined => {
    return commands.find(cmd => cmd.name.toLowerCase() === name.toLowerCase());
  }, [commands]);

  // Expand a command's prompt (replace variables, etc.)
  const expandCommandPrompt = useCallback((command: CustomCommand, context?: Record<string, string>): string => {
    let expandedPrompt = command.prompt;

    // Replace common variables if context is provided
    if (context) {
      Object.entries(context).forEach(([key, value]) => {
        const placeholder = `{${key}}`;
        expandedPrompt = expandedPrompt.replace(new RegExp(placeholder, 'g'), value);
      });
    }

    return expandedPrompt;
  }, []);

  // Increment usage count for a command
  const incrementUsage = useCallback(async (commandId: string) => {
    const updatedCommands = commands.map(cmd =>
      cmd.id === commandId
        ? { ...cmd, usageCount: cmd.usageCount + 1, updatedAt: new Date() }
        : cmd
    );
    await saveCommands(updatedCommands);
  }, [commands, saveCommands]);

  // Add a new command
  const addCommand = useCallback(async (commandData: Omit<CustomCommand, 'id' | 'createdAt' | 'updatedAt'>) => {
    const newCommand: CustomCommand = {
      ...commandData,
      id: `cmd_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      createdAt: new Date(),
      updatedAt: new Date(),
    };
    
    const updatedCommands = [...commands, newCommand];
    await saveCommands(updatedCommands);
    return newCommand;
  }, [commands, saveCommands]);

  // Update an existing command
  const updateCommand = useCallback(async (commandId: string, updates: Partial<CustomCommand>) => {
    const updatedCommands = commands.map(cmd =>
      cmd.id === commandId
        ? { ...cmd, ...updates, updatedAt: new Date() }
        : cmd
    );
    await saveCommands(updatedCommands);
  }, [commands, saveCommands]);

  // Delete a command
  const deleteCommand = useCallback(async (commandId: string) => {
    const updatedCommands = commands.filter(cmd => cmd.id !== commandId);
    await saveCommands(updatedCommands);
  }, [commands, saveCommands]);

  // Toggle favorite status
  const toggleFavorite = useCallback(async (commandId: string) => {
    const updatedCommands = commands.map(cmd =>
      cmd.id === commandId
        ? { ...cmd, isFavorite: !cmd.isFavorite, updatedAt: new Date() }
        : cmd
    );
    await saveCommands(updatedCommands);
  }, [commands, saveCommands]);

  // Load commands on mount
  useEffect(() => {
    loadCommands();
  }, [loadCommands]);

  return {
    commands,
    isLoading,
    error,
    loadCommands,
    getCommand,
    getCommandByName,
    expandCommandPrompt,
    incrementUsage,
    addCommand,
    updateCommand,
    deleteCommand,
    toggleFavorite,
  };
};

// Example commands for demo
const getExampleCommands = (): CustomCommand[] => [
  {
    id: 'cmd_example_1',
    name: 'document',
    label: 'Create Document',
    description: 'Generate comprehensive documentation for code or projects',
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
    createdAt: new Date(),
    updatedAt: new Date(),
    usageCount: 12,
    isFavorite: true,
  },
  {
    id: 'cmd_example_2',
    name: 'review',
    label: 'Code Review',
    description: 'Perform thorough code review with suggestions and best practices',
    prompt: `Please perform a comprehensive code review of the provided code. Focus on:

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
    createdAt: new Date(),
    updatedAt: new Date(),
    usageCount: 8,
    isFavorite: false,
  },
  {
    id: 'cmd_example_3',
    name: 'explain',
    label: 'Explain Code',
    description: 'Provide detailed explanation of how code works',
    prompt: `Please explain the provided code in detail. Include:

1. What the code does (high-level purpose)
2. How it works (step-by-step breakdown)
3. Key concepts and patterns used
4. Dependencies and requirements
5. Potential use cases
6. Any notable design decisions

Make the explanation accessible to developers who may not be familiar with this specific code.`,
    icon: 'Code',
    category: 'development',
    variables: [],
    createdAt: new Date(),
    updatedAt: new Date(),
    usageCount: 15,
    isFavorite: true,
  },
  {
    id: 'cmd_example_4',
    name: 'optimize',
    label: 'Optimize Performance',
    description: 'Analyze and suggest performance optimizations',
    prompt: `Please analyze the provided code for performance optimization opportunities. Focus on:

1. Algorithmic efficiency improvements
2. Memory usage optimization
3. Database query optimization (if applicable)
4. Caching strategies
5. Bottleneck identification
6. Scalability considerations

Provide specific recommendations with code examples where possible.`,
    icon: 'Zap',
    category: 'development',
    variables: [],
    createdAt: new Date(),
    updatedAt: new Date(),
    usageCount: 5,
    isFavorite: false,
  }
];

export default useCustomCommands;
