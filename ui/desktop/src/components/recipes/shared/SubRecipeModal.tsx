import React, { useState, useEffect } from 'react';
import { X, Plus, Trash2, FolderOpen } from 'lucide-react';
import { Button } from '../../ui/button';
import { SubRecipeFormData } from './recipeFormSchema';
import { useEscapeKey } from '../../../hooks/useEscapeKey';

interface SubRecipeModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSave: (subRecipe: SubRecipeFormData) => void;
  subRecipe?: SubRecipeFormData | null;
}

export default function SubRecipeModal({
  isOpen,
  onClose,
  onSave,
  subRecipe,
}: SubRecipeModalProps) {
  const [name, setName] = useState('');
  const [path, setPath] = useState('');
  const [description, setDescription] = useState('');
  const [sequentialWhenRepeated, setSequentialWhenRepeated] = useState(false);
  const [values, setValues] = useState<Record<string, string>>({});
  const [newValueKey, setNewValueKey] = useState('');
  const [newValueValue, setNewValueValue] = useState('');

  useEscapeKey(isOpen, onClose);

  useEffect(() => {
    if (isOpen) {
      if (subRecipe) {
        setName(subRecipe.name);
        setPath(subRecipe.path);
        setDescription(subRecipe.description || '');
        setSequentialWhenRepeated(subRecipe.sequential_when_repeated ?? false);
        setValues(subRecipe.values || {});
      } else {
        setName('');
        setPath('');
        setDescription('');
        setSequentialWhenRepeated(false);
        setValues({});
      }
      setNewValueKey('');
      setNewValueValue('');
    }
  }, [isOpen, subRecipe]);

  const handleSave = () => {
    if (!name.trim() || !path.trim()) {
      return;
    }

    const subRecipeData: SubRecipeFormData = {
      name: name.trim(),
      path: path.trim(),
      description: description.trim() || undefined,
      sequential_when_repeated: sequentialWhenRepeated,
      values: Object.keys(values).length > 0 ? values : undefined,
    };

    onSave(subRecipeData);
    onClose();
  };

  const handleAddValue = () => {
    if (newValueKey.trim() && newValueValue.trim()) {
      setValues({ ...values, [newValueKey.trim()]: newValueValue.trim() });
      setNewValueKey('');
      setNewValueValue('');
    }
  };

  const handleRemoveValue = (key: string) => {
    const newValues = { ...values };
    delete newValues[key];
    setValues(newValues);
  };

  const handleKeyPress = (e: React.KeyboardEvent, action: () => void) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      action();
    }
  };

  const handleBrowseFile = async () => {
    try {
      const selectedPath = await window.electron.selectFileOrDirectory();
      if (selectedPath) {
        setPath(selectedPath);
      }
    } catch (error) {
      console.error('Failed to browse for file:', error);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-[500] flex items-center justify-center bg-black/50">
      <div className="bg-background-default border border-borderSubtle rounded-lg w-[90vw] max-w-2xl max-h-[90vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b border-borderSubtle">
          <div>
            <h2 className="text-xl font-medium text-textProminent">
              {subRecipe ? 'Configure Subrecipe' : 'Add Subrecipe'}
            </h2>
            <p className="text-textSubtle text-sm">
              Configure a subrecipe that can be called as a tool during recipe execution
            </p>
          </div>
          <Button
            onClick={onClose}
            variant="ghost"
            size="sm"
            className="p-2 hover:bg-bgSubtle rounded-lg transition-colors"
          >
            <X className="w-5 h-5" />
          </Button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto px-6 py-4 space-y-4">
          {/* Name Field */}
          <div>
            <label
              htmlFor="subrecipe-name"
              className="block text-sm font-medium text-text-standard mb-2"
            >
              Name <span className="text-red-500">*</span>
            </label>
            <input
              id="subrecipe-name"
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="w-full p-3 border border-border-subtle rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500"
              placeholder="e.g., security_scan"
            />
            <p className="text-xs text-text-muted mt-1">
              Unique identifier used to generate the tool name
            </p>
          </div>

          {/* Path Field */}
          <div>
            <label
              htmlFor="subrecipe-path"
              className="block text-sm font-medium text-text-standard mb-2"
            >
              Path <span className="text-red-500">*</span>
            </label>
            <div className="flex gap-2">
              <input
                id="subrecipe-path"
                type="text"
                value={path}
                onChange={(e) => setPath(e.target.value)}
                className="flex-1 p-3 border border-border-subtle rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500"
                placeholder="e.g., ./subrecipes/security-analysis.yaml"
              />
              <Button
                type="button"
                onClick={handleBrowseFile}
                variant="outline"
                className="px-4 py-2 flex items-center gap-2"
              >
                <FolderOpen className="w-4 h-4" />
                Browse
              </Button>
            </div>
            <p className="text-xs text-text-muted mt-1">
              Browse for an existing recipe file or enter a path manually
            </p>
          </div>

          {/* Description Field */}
          <div>
            <label
              htmlFor="subrecipe-description"
              className="block text-sm font-medium text-text-standard mb-2"
            >
              Description
            </label>
            <textarea
              id="subrecipe-description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              className="w-full p-3 border border-border-subtle rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
              placeholder="Optional description of what this subrecipe does..."
              rows={3}
            />
          </div>

          {/* Sequential When Repeated */}
          <div className="flex items-center gap-2">
            <input
              id="subrecipe-sequential"
              type="checkbox"
              checked={sequentialWhenRepeated}
              onChange={(e) => setSequentialWhenRepeated(e.target.checked)}
              className="w-4 h-4 text-blue-500 border-border-subtle rounded focus:ring-2 focus:ring-blue-500"
            />
            <label htmlFor="subrecipe-sequential" className="text-sm text-text-standard">
              Sequential when repeated
            </label>
            <span className="text-xs text-text-muted">
              (Forces sequential execution of multiple subrecipe instances)
            </span>
          </div>

          {/* Values Section */}
          <div>
            <label className="block text-sm font-medium text-text-standard mb-2">
              Pre-configured Values
            </label>
            <p className="text-xs text-text-muted mb-3">
              Optional parameter values that are always passed to the subrecipe
            </p>

            {/* Add Value Input */}
            <div className="flex gap-2 mb-3">
              <input
                type="text"
                value={newValueKey}
                onChange={(e) => setNewValueKey(e.target.value)}
                onKeyPress={(e) => handleKeyPress(e, handleAddValue)}
                placeholder="Parameter name..."
                className="flex-1 px-3 py-2 border border-border-subtle rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm"
              />
              <input
                type="text"
                value={newValueValue}
                onChange={(e) => setNewValueValue(e.target.value)}
                onKeyPress={(e) => handleKeyPress(e, handleAddValue)}
                placeholder="Parameter value..."
                className="flex-1 px-3 py-2 border border-border-subtle rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm"
              />
              <Button
                type="button"
                onClick={handleAddValue}
                disabled={!newValueKey.trim() || !newValueValue.trim()}
                variant="outline"
                size="sm"
                className="px-3"
              >
                <Plus className="w-4 h-4" />
              </Button>
            </div>

            {/* Values List */}
            {Object.keys(values).length > 0 && (
              <div className="space-y-2 border border-border-subtle rounded-lg p-3">
                {Object.entries(values).map(([key, value]) => (
                  <div
                    key={key}
                    className="flex items-center justify-between p-2 bg-background-muted rounded"
                  >
                    <div className="flex-1">
                      <span className="text-sm font-medium text-text-standard">{key}</span>
                      <span className="text-sm text-text-muted mx-2">=</span>
                      <span className="text-sm text-text-standard">{value}</span>
                    </div>
                    <Button
                      type="button"
                      onClick={() => handleRemoveValue(key)}
                      variant="ghost"
                      size="sm"
                      className="p-1 hover:bg-red-100 hover:text-red-600"
                    >
                      <Trash2 className="w-4 h-4" />
                    </Button>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>

        {/* Footer */}
        <div className="flex gap-2 p-6 border-t border-borderSubtle">
          <Button onClick={onClose} variant="outline" className="flex-1">
            Cancel
          </Button>
          <Button
            onClick={handleSave}
            disabled={!name.trim() || !path.trim()}
            className="flex-1 bg-blue-500 hover:bg-blue-600 text-white"
          >
            {subRecipe ? 'Apply' : 'Add Subrecipe'}
          </Button>
        </div>
      </div>
    </div>
  );
}
