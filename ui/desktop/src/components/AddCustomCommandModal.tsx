import React, { useState, useEffect } from 'react';
import { Plus, Zap, Code, FileText, Search, Play, Settings, Hash } from 'lucide-react';
import { CustomCommand } from '../types/customCommands';
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from './ui/dialog';
import { Button } from './ui/button';

interface AddCustomCommandModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSave: (command: CustomCommand) => void;
  editingCommand?: CustomCommand | null;
}

const iconOptions = [
  { name: 'Zap', icon: <Zap size={16} /> },
  { name: 'Code', icon: <Code size={16} /> },
  { name: 'FileText', icon: <FileText size={16} /> },
  { name: 'Search', icon: <Search size={16} /> },
  { name: 'Play', icon: <Play size={16} /> },
  { name: 'Settings', icon: <Settings size={16} /> },
  { name: 'Hash', icon: <Hash size={16} /> },
];

export const AddCustomCommandModal: React.FC<AddCustomCommandModalProps> = ({
  isOpen,
  onClose,
  onSave,
  editingCommand,
}) => {
  const [formData, setFormData] = useState({
    name: '',
    label: '',
    description: '',
    prompt: '',
    icon: 'Zap',
  });
  const [errors, setErrors] = useState<Record<string, string>>({});

  // Reset form when modal opens/closes or editing command changes
  useEffect(() => {
    if (isOpen) {
      if (editingCommand) {
        setFormData({
          name: editingCommand.name,
          label: editingCommand.label,
          description: editingCommand.description,
          prompt: editingCommand.prompt,
          icon: editingCommand.icon || 'Zap',
        });
      } else {
        setFormData({
          name: '',
          label: '',
          description: '',
          prompt: '',
          icon: 'Zap',
        });
      }
      setErrors({});
    }
  }, [isOpen, editingCommand]);

  const validateForm = (): boolean => {
    const newErrors: Record<string, string> = {};

    if (!formData.name.trim()) {
      newErrors.name = 'Command name is required';
    } else if (!/^[a-zA-Z0-9-_]+$/.test(formData.name)) {
      newErrors.name = 'Command name can only contain letters, numbers, hyphens, and underscores';
    }

    if (!formData.label.trim()) {
      newErrors.label = 'Display label is required';
    }

    if (!formData.description.trim()) {
      newErrors.description = 'Description is required';
    }

    if (!formData.prompt.trim()) {
      newErrors.prompt = 'Prompt is required';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!validateForm()) {
      return;
    }

    const command: CustomCommand = {
      id: editingCommand?.id || `cmd_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      name: formData.name.trim(),
      label: formData.label.trim(),
      description: formData.description.trim(),
      prompt: formData.prompt.trim(),
      icon: formData.icon,
      category: editingCommand?.category || undefined,
      createdAt: editingCommand?.createdAt || new Date(),
      updatedAt: new Date(),
      usageCount: editingCommand?.usageCount || 0,
      isFavorite: editingCommand?.isFavorite || false,
    };

    onSave(command);
    onClose();
  };

  const handleInputChange = (field: string, value: string) => {
    setFormData(prev => ({ ...prev, [field]: value }));
    
    // Auto-generate label from name if label is empty
    if (field === 'name' && !formData.label) {
      const autoLabel = value
        .replace(/[-_]/g, ' ')
        .replace(/\b\w/g, l => l.toUpperCase())
        .trim();
      setFormData(prev => ({ ...prev, label: autoLabel }));
    }
    
    // Clear error when user starts typing
    if (errors[field]) {
      setErrors(prev => ({ ...prev, [field]: '' }));
    }
  };

  return (
    <Dialog
      open={isOpen}
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
    >
      <DialogContent className="sm:max-w-[600px] max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>
            {editingCommand ? 'Edit Custom Command' : 'Add Custom Command'}
          </DialogTitle>
        </DialogHeader>

        <div className="py-4">
          <form onSubmit={handleSubmit} className="space-y-6">
            {/* Command Name */}
            <div>
              <label className="block text-sm font-medium text-textStandard mb-2">
                Command Name *
              </label>
              <div className="relative">
                <span className="absolute left-3 top-1/2 transform -translate-y-1/2 text-textSubtle">
                  /
                </span>
                <input
                  type="text"
                  value={formData.name}
                  onChange={(e) => handleInputChange('name', e.target.value)}
                  placeholder="document"
                  className={`w-full pl-8 pr-3 py-2 border rounded-md bg-background-default text-textStandard placeholder-textSubtle ${
                    errors.name ? 'border-red-500' : 'border-borderStandard'
                  } focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent`}
                />
              </div>
              {errors.name && (
                <p className="mt-1 text-sm text-red-500">{errors.name}</p>
              )}
              <p className="mt-1 text-xs text-gray-500">
                This will be the command users type (e.g., /document)
              </p>
            </div>

            {/* Display Label */}
            <div>
              <label className="block text-sm font-medium text-textStandard mb-2">
                Display Label *
              </label>
              <input
                type="text"
                value={formData.label}
                onChange={(e) => handleInputChange('label', e.target.value)}
                placeholder="Create Document"
                className={`w-full px-3 py-2 border rounded-md bg-background-default text-textStandard placeholder-textSubtle ${
                  errors.label ? 'border-red-500' : 'border-borderStandard'
                } focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent`}
              />
              {errors.label && (
                <p className="mt-1 text-sm text-red-500">{errors.label}</p>
              )}
              <p className="mt-1 text-xs text-gray-500">
                Friendly name shown in the command list
              </p>
            </div>

            {/* Description */}
            <div>
              <label className="block text-sm font-medium text-textStandard mb-2">
                Description *
              </label>
              <input
                type="text"
                value={formData.description}
                onChange={(e) => handleInputChange('description', e.target.value)}
                placeholder="Create a new document with proper formatting"
                className={`w-full px-3 py-2 border rounded-md bg-background-default text-textStandard placeholder-textSubtle ${
                  errors.description ? 'border-red-500' : 'border-borderStandard'
                } focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent`}
              />
              {errors.description && (
                <p className="mt-1 text-sm text-red-500">{errors.description}</p>
              )}
            </div>

            {/* Icon Selection */}
            <div>
              <label className="block text-sm font-medium text-textStandard mb-2">
                Icon
              </label>
              <div className="flex flex-wrap gap-2">
                {iconOptions.map((option) => (
                  <button
                    key={option.name}
                    type="button"
                    onClick={() => handleInputChange('icon', option.name)}
                    className={`p-2 rounded-md border transition-colors ${
                      formData.icon === option.name
                        ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
                        : 'border-borderStandard hover:bg-bgSubtle'
                    }`}
                  >
                    {option.icon}
                  </button>
                ))}
              </div>
            </div>



            {/* Prompt */}
            <div>
              <label className="block text-sm font-medium text-textStandard mb-2">
                Prompt *
              </label>
              <textarea
                value={formData.prompt}
                onChange={(e) => handleInputChange('prompt', e.target.value)}
                placeholder="Please create a comprehensive document about the topic provided. Include an introduction, main sections with detailed explanations, and a conclusion. Use proper markdown formatting with headers, bullet points, and code blocks where appropriate."
                rows={6}
                className={`w-full px-3 py-2 border rounded-md bg-background-default text-textStandard placeholder-textSubtle resize-vertical ${
                  errors.prompt ? 'border-red-500' : 'border-borderStandard'
                } focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent`}
              />
              {errors.prompt && (
                <p className="mt-1 text-sm text-red-500">{errors.prompt}</p>
              )}
              <p className="mt-1 text-xs text-gray-500">
                This is the full prompt that will be sent to the AI when the command is used
              </p>
            </div>


          </form>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={handleSubmit} className="flex items-center gap-2">
            <Plus size={16} />
            {editingCommand ? 'Update Command' : 'Add Command'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export default AddCustomCommandModal;
