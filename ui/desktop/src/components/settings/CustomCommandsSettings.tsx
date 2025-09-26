import React, { useState, useEffect } from 'react';
import { Plus, Edit, Trash2, Copy, Download, Upload, Star, StarOff, Zap, Code, FileText, Search } from 'lucide-react';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { AddCustomCommandModal } from '../AddCustomCommandModal';
import { 
  CustomCommand, 
  CustomCommandCategory, 
  DEFAULT_CATEGORIES
} from '../../types/customCommands';

interface CustomCommandsSettingsProps {
  onClose?: () => void;
}

const ICON_MAP = {
  'Zap': <Zap size={16} />,
  'Code': <Code size={16} />,
  'FileText': <FileText size={16} />,
  'Search': <Search size={16} />,
};

export const CustomCommandsSettings: React.FC<CustomCommandsSettingsProps> = () => {
  const [commands, setCommands] = useState<CustomCommand[]>([]);
  const [categories] = useState<CustomCommandCategory[]>(DEFAULT_CATEGORIES);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingCommand, setEditingCommand] = useState<CustomCommand | null>(null);
  const [searchQuery, setSearchQuery] = useState('');

  // Load commands from storage on mount
  useEffect(() => {
    loadCommands();
  }, []);

  const loadCommands = async () => {
    try {
      // TODO: Load from actual storage (localStorage, config API, etc.)
      const stored = localStorage.getItem('goose-custom-commands');
      if (stored) {
        const parsed = JSON.parse(stored);
        setCommands(parsed.map((cmd: any) => ({
          ...cmd,
          createdAt: new Date(cmd.createdAt),
          updatedAt: new Date(cmd.updatedAt)
        })));
      } else {
        // Load some example commands for demo
        setCommands(getExampleCommands());
      }
    } catch (error) {
      console.error('Failed to load custom commands:', error);
    }
  };

  const saveCommands = async (updatedCommands: CustomCommand[]) => {
    try {
      localStorage.setItem('goose-custom-commands', JSON.stringify(updatedCommands));
      setCommands(updatedCommands);
    } catch (error) {
      console.error('Failed to save custom commands:', error);
    }
  };

  const handleModalSave = (command: CustomCommand) => {
    const now = new Date();
    let updatedCommands: CustomCommand[];

    if (editingCommand) {
      // Update existing command
      updatedCommands = commands.map(cmd => 
        cmd.id === editingCommand.id 
          ? { ...command, id: editingCommand.id, createdAt: editingCommand.createdAt, updatedAt: now }
          : cmd
      );
    } else {
      // Create new command
      updatedCommands = [...commands, { ...command, createdAt: now, updatedAt: now }];
    }

    saveCommands(updatedCommands);
  };

  const handleEdit = (command: CustomCommand) => {
    setEditingCommand(command);
    setIsModalOpen(true);
  };

  const handleCreateNew = () => {
    setEditingCommand(null);
    setIsModalOpen(true);
  };

  const handleCloseModal = () => {
    setIsModalOpen(false);
    setEditingCommand(null);
  };

  const handleDelete = (commandId: string) => {
    if (confirm('Are you sure you want to delete this command?')) {
      const updatedCommands = commands.filter(cmd => cmd.id !== commandId);
      saveCommands(updatedCommands);
    }
  };

  const handleToggleFavorite = (commandId: string) => {
    const updatedCommands = commands.map(cmd =>
      cmd.id === commandId ? { ...cmd, isFavorite: !cmd.isFavorite } : cmd
    );
    saveCommands(updatedCommands);
  };

  const handleDuplicate = (command: CustomCommand) => {
    const duplicatedCommand: CustomCommand = {
      ...command,
      id: `cmd_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      name: `${command.name}_copy`,
      label: `${command.label} (Copy)`,
      createdAt: new Date(),
      updatedAt: new Date(),
      usageCount: 0,
    };
    saveCommands([...commands, duplicatedCommand]);
  };

  const filteredCommands = commands.filter(cmd => {
    const matchesSearch = !searchQuery || 
      cmd.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      cmd.label.toLowerCase().includes(searchQuery.toLowerCase()) ||
      cmd.description.toLowerCase().includes(searchQuery.toLowerCase());
    
    return matchesSearch;
  });

  return (
    <>
      <div className="space-y-1">
        {/* Header Actions */}
        <div className="flex items-center justify-between gap-4 mb-4 px-2">
          <Input
            placeholder="Search commands..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-64"
          />
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" className="p-2">
              <Upload size={16} />
            </Button>
            <Button variant="outline" size="sm" className="p-2">
              <Download size={16} />
            </Button>
            <Button onClick={handleCreateNew} size="sm" className="p-2">
              <Plus size={16} />
            </Button>
          </div>
        </div>

        {/* Commands List - Row Style */}
        {filteredCommands.map(command => (
          <div key={command.id} className="group hover:cursor-pointer text-sm">
            <div className="flex items-center justify-between text-text-default py-2 px-2 bg-background-default hover:bg-background-muted rounded-lg transition-all">
              <div className="flex items-center gap-3">
                <div className="flex items-center justify-center text-text-muted">
                  {ICON_MAP[command.icon as keyof typeof ICON_MAP] || ICON_MAP.Zap}
                </div>
                <div>
                  <h3 className="text-text-default font-medium">/{command.name}</h3>
                  <p className="text-text-muted text-xs mt-[2px]">{command.description}</p>
                </div>
              </div>

              <div className="flex items-center gap-2">
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={(e) => {
                    e.stopPropagation();
                    handleToggleFavorite(command.id);
                  }}
                  className="p-1 h-6 w-6"
                >
                  {command.isFavorite ? (
                    <Star size={12} className="text-yellow-500 fill-yellow-500" />
                  ) : (
                    <StarOff size={12} className="text-text-muted" />
                  )}
                </Button>
                <Button 
                  variant="ghost" 
                  size="sm" 
                  onClick={(e) => {
                    e.stopPropagation();
                    handleEdit(command);
                  }}
                  className="p-1 h-6 w-6"
                >
                  <Edit size={12} className="text-text-muted hover:text-text-default" />
                </Button>
                <Button 
                  variant="ghost" 
                  size="sm" 
                  onClick={(e) => {
                    e.stopPropagation();
                    handleDuplicate(command);
                  }}
                  className="p-1 h-6 w-6"
                >
                  <Copy size={12} className="text-text-muted hover:text-text-default" />
                </Button>
                <Button 
                  variant="ghost" 
                  size="sm" 
                  onClick={(e) => {
                    e.stopPropagation();
                    handleDelete(command.id);
                  }}
                  className="p-1 h-6 w-6 text-red-600 hover:text-red-700"
                >
                  <Trash2 size={12} />
                </Button>
              </div>
            </div>
          </div>
        ))}

        {filteredCommands.length === 0 && (
          <div className="text-center py-8">
            <div className="w-12 h-12 bg-background-muted rounded-full flex items-center justify-center mx-auto mb-3">
              <Zap size={16} className="text-text-muted" />
            </div>
            <h3 className="text-sm font-medium text-text-default mb-1">
              {searchQuery ? 'No commands found' : 'No custom commands yet'}
            </h3>
            <p className="text-text-muted text-xs mb-3">
              {searchQuery 
                ? 'Try adjusting your search query'
                : 'Create your first custom slash command to get started'
              }
            </p>
            {!searchQuery && (
              <Button onClick={handleCreateNew} size="sm">
                <Plus size={14} className="mr-2" />
                Create Command
              </Button>
            )}
          </div>
        )}
      </div>

      {/* Modal */}
      <AddCustomCommandModal
        isOpen={isModalOpen}
        onClose={handleCloseModal}
        onSave={handleModalSave}
        editingCommand={editingCommand}
      />
    </>
  );
};

// Example commands for demo - these provide useful starting points for users
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
    usageCount: 0,
    isFavorite: false,
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
    usageCount: 0,
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
    usageCount: 0,
    isFavorite: false,
  },
  {
    id: 'cmd_example_4',
    name: 'optimize',
    label: 'Optimize Code',
    description: 'Suggest optimizations and improvements for better performance',
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
    createdAt: new Date(),
    updatedAt: new Date(),
    usageCount: 0,
    isFavorite: false,
  },
  {
    id: 'cmd_example_5',
    name: 'test',
    label: 'Generate Tests',
    description: 'Create comprehensive unit tests for the provided code',
    prompt: `Please generate comprehensive unit tests for the provided code. Include:

1. Test cases for normal/expected behavior
2. Edge cases and boundary conditions
3. Error handling and exception cases
4. Mock objects where appropriate
5. Test setup and teardown if needed
6. Clear, descriptive test names

Use the appropriate testing framework for the language and follow testing best practices.`,
    icon: 'Search',
    category: 'testing',
    variables: [],
    createdAt: new Date(),
    updatedAt: new Date(),
    usageCount: 0,
    isFavorite: false,
  }
];

export default CustomCommandsSettings;
