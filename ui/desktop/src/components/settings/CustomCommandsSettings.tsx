import React, { useState, useEffect } from 'react';
import { Plus, Edit, Trash2, Copy, Download, Upload, Star, StarOff, Zap, Code, FileText, Search } from 'lucide-react';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Textarea } from '../ui/textarea';
import { Label } from '../ui/label';
import { 
  CustomCommand, 
  CustomCommandCategory, 
  COMMAND_TEMPLATE, 
  DEFAULT_CATEGORIES,
  COMMAND_VALIDATION
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
  const [isEditing, setIsEditing] = useState(false);
  const [editingCommand, setEditingCommand] = useState<CustomCommand | null>(null);
  const [formData, setFormData] = useState(COMMAND_TEMPLATE);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<string>('all');
  const [errors, setErrors] = useState<Record<string, string>>({});

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

  const validateForm = (): boolean => {
    const newErrors: Record<string, string> = {};

    // Validate name
    if (!formData.name) {
      newErrors.name = 'Command name is required';
    } else if (formData.name.length < COMMAND_VALIDATION.name.minLength) {
      newErrors.name = `Name must be at least ${COMMAND_VALIDATION.name.minLength} characters`;
    } else if (!COMMAND_VALIDATION.name.pattern.test(formData.name)) {
      newErrors.name = 'Name must start with a letter and contain only letters and numbers';
    } else if (commands.some(cmd => cmd.name.toLowerCase() === formData.name.toLowerCase() && cmd.id !== editingCommand?.id)) {
      newErrors.name = 'A command with this name already exists';
    }

    // Validate label
    if (!formData.label) {
      newErrors.label = 'Label is required';
    } else if (formData.label.length < COMMAND_VALIDATION.label.minLength) {
      newErrors.label = `Label must be at least ${COMMAND_VALIDATION.label.minLength} characters`;
    }

    // Validate description
    if (!formData.description) {
      newErrors.description = 'Description is required';
    } else if (formData.description.length < COMMAND_VALIDATION.description.minLength) {
      newErrors.description = `Description must be at least ${COMMAND_VALIDATION.description.minLength} characters`;
    }

    // Validate prompt
    if (!formData.prompt) {
      newErrors.prompt = 'Prompt is required';
    } else if (formData.prompt.length < COMMAND_VALIDATION.prompt.minLength) {
      newErrors.prompt = `Prompt must be at least ${COMMAND_VALIDATION.prompt.minLength} characters`;
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSave = () => {
    if (!validateForm()) return;

    const now = new Date();
    let updatedCommands: CustomCommand[];

    if (editingCommand) {
      // Update existing command
      updatedCommands = commands.map(cmd => 
        cmd.id === editingCommand.id 
          ? { ...formData, id: editingCommand.id, createdAt: editingCommand.createdAt, updatedAt: now }
          : cmd
      );
    } else {
      // Create new command
      const newCommand: CustomCommand = {
        ...formData,
        id: `cmd_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
        createdAt: now,
        updatedAt: now,
      };
      updatedCommands = [...commands, newCommand];
    }

    saveCommands(updatedCommands);
    setIsEditing(false);
    setEditingCommand(null);
    setFormData(COMMAND_TEMPLATE);
    setErrors({});
  };

  const handleEdit = (command: CustomCommand) => {
    setEditingCommand(command);
    setFormData({
      name: command.name,
      label: command.label,
      description: command.description,
      prompt: command.prompt,
      icon: command.icon || 'Zap',
      category: command.category || 'general',
      variables: command.variables || [],
      usageCount: command.usageCount,
      isFavorite: command.isFavorite,
    });
    setIsEditing(true);
    setErrors({});
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
    
    const matchesCategory = selectedCategory === 'all' || cmd.category === selectedCategory;
    
    return matchesSearch && matchesCategory;
  });

  const handleCancel = () => {
    setIsEditing(false);
    setEditingCommand(null);
    setFormData(COMMAND_TEMPLATE);
    setErrors({});
  };

  return (
    <div className="space-y-1">
      {!isEditing ? (
        <>
          {/* Header Actions */}
          <div className="flex items-center justify-between gap-4 mb-4">
            <div className="flex items-center gap-4">
              <Input
                placeholder="Search commands..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-64"
              />
              <select
                value={selectedCategory}
                onChange={(e) => setSelectedCategory(e.target.value)}
                className="px-3 py-2 border border-border-default rounded-md bg-background-default text-text-default"
              >
                <option value="all">All Categories</option>
                {categories.map(cat => (
                  <option key={cat.id} value={cat.id}>{cat.name}</option>
                ))}
              </select>
            </div>
            <div className="flex items-center gap-2">
              <Button variant="outline" size="sm">
                <Upload size={16} className="mr-2" />
                Import
              </Button>
              <Button variant="outline" size="sm">
                <Download size={16} className="mr-2" />
                Export
              </Button>
              <Button onClick={() => setIsEditing(true)}>
                <Plus size={16} className="mr-2" />
                New Command
              </Button>
            </div>
          </div>

          {/* Commands List - Row Style */}
          {filteredCommands.map(command => (
            <div key={command.id} className="group hover:cursor-pointer text-sm">
              <div className="flex items-center justify-between text-text-default py-2 px-2 bg-background-default hover:bg-background-muted rounded-lg transition-all">
                <div className="flex items-center gap-3">
                  <div className="w-6 h-6 bg-background-muted rounded-full flex items-center justify-center">
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
                {searchQuery || selectedCategory !== 'all' ? 'No commands found' : 'No custom commands yet'}
              </h3>
              <p className="text-text-muted text-xs mb-3">
                {searchQuery || selectedCategory !== 'all' 
                  ? 'Try adjusting your search or category filter'
                  : 'Create your first custom slash command to get started'
                }
              </p>
              {!searchQuery && selectedCategory === 'all' && (
                <Button onClick={() => setIsEditing(true)} size="sm">
                  <Plus size={14} className="mr-2" />
                  Create Command
                </Button>
              )}
            </div>
          )}
        </>
      ) : (
        /* Command Editor */
        <div className="max-w-2xl mx-auto space-y-6">
          <div className="flex items-center justify-between">
            <h3 className="text-xl font-semibold text-textStandard">
              {editingCommand ? 'Edit Command' : 'Create New Command'}
            </h3>
            <div className="flex items-center gap-2">
              <Button variant="outline" onClick={handleCancel}>
                Cancel
              </Button>
              <Button onClick={handleSave}>
                {editingCommand ? 'Update' : 'Create'} Command
              </Button>
            </div>
          </div>

          <div className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div>
                <Label htmlFor="name">Command Name *</Label>
                <Input
                  id="name"
                  placeholder="e.g., document, review, explain"
                  value={formData.name}
                  onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                  className={errors.name ? 'border-red-500' : ''}
                />
                {errors.name && <p className="text-red-500 text-sm mt-1">{errors.name}</p>}
                <p className="text-textSubtle text-xs mt-1">
                  Users will type /{formData.name || 'name'} to use this command
                </p>
              </div>

              <div>
                <Label htmlFor="label">Display Label *</Label>
                <Input
                  id="label"
                  placeholder="e.g., Create Document, Code Review"
                  value={formData.label}
                  onChange={(e) => setFormData({ ...formData, label: e.target.value })}
                  className={errors.label ? 'border-red-500' : ''}
                />
                {errors.label && <p className="text-red-500 text-sm mt-1">{errors.label}</p>}
              </div>
            </div>

            <div>
              <Label htmlFor="description">Description *</Label>
              <Input
                id="description"
                placeholder="Brief description of what this command does"
                value={formData.description}
                onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                className={errors.description ? 'border-red-500' : ''}
              />
              {errors.description && <p className="text-red-500 text-sm mt-1">{errors.description}</p>}
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div>
                <Label htmlFor="category">Category</Label>
                <select
                  id="category"
                  value={formData.category}
                  onChange={(e) => setFormData({ ...formData, category: e.target.value })}
                  className="w-full px-3 py-2 border border-borderStandard rounded-md bg-background-default text-textStandard"
                >
                  {categories.map(cat => (
                    <option key={cat.id} value={cat.id}>{cat.name}</option>
                  ))}
                </select>
              </div>

              <div>
                <Label htmlFor="icon">Icon</Label>
                <select
                  id="icon"
                  value={formData.icon}
                  onChange={(e) => setFormData({ ...formData, icon: e.target.value })}
                  className="w-full px-3 py-2 border border-borderStandard rounded-md bg-background-default text-textStandard"
                >
                  {Object.keys(ICON_MAP).map(iconName => (
                    <option key={iconName} value={iconName}>{iconName}</option>
                  ))}
                </select>
              </div>
            </div>

            <div>
              <Label htmlFor="prompt">Prompt Template *</Label>
              <Textarea
                id="prompt"
                placeholder="Enter the full prompt that will be sent to the AI when this command is used..."
                value={formData.prompt}
                onChange={(e) => setFormData({ ...formData, prompt: e.target.value })}
                rows={8}
                className={errors.prompt ? 'border-red-500' : ''}
              />
              {errors.prompt && <p className="text-red-500 text-sm mt-1">{errors.prompt}</p>}
              <p className="text-textSubtle text-xs mt-1">
                This is the actual prompt that will be sent to the AI. Users will see [{formData.label || 'Label'}] as a pill in their input.
              </p>
            </div>

            {/* Preview */}
            <div className="border border-borderStandard rounded-lg p-4 bg-bgSubtle">
              <h4 className="font-medium text-textStandard mb-2">Preview</h4>
              <div className="flex items-center gap-2 mb-2">
                <span className="text-textSubtle">User types:</span>
                <code className="bg-background-default px-2 py-1 rounded text-sm">/{formData.name || 'name'}</code>
              </div>
              <div className="flex items-center gap-2 mb-2">
                <span className="text-textSubtle">User sees:</span>
                <div className="inline-flex items-center gap-1.5 px-2 py-1 bg-bgProminent text-textProminentInverse border border-borderProminent rounded-full text-xs font-medium">
                  <span className="flex items-center gap-1">
                    <span className="relative">
                      <div className="w-3 h-3 bg-blue-500 rounded-full absolute inset-0" />
                      <span className="relative text-white text-[8px] flex items-center justify-center w-3 h-3">
                        {ICON_MAP[formData.icon as keyof typeof ICON_MAP] || ICON_MAP.Zap}
                      </span>
                    </span>
                    {formData.label || 'Label'}
                  </span>
                </div>
              </div>
              <div className="text-textSubtle text-sm">
                <span>AI receives:</span>
                <div className="bg-background-default p-2 rounded mt-1 text-xs font-mono max-h-32 overflow-y-auto">
                  {formData.prompt || 'Prompt template will appear here...'}
                </div>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
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
