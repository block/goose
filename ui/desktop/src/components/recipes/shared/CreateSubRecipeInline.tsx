import React, { useState } from 'react';
import { X, Save, Loader2, Plus, Trash2 } from 'lucide-react';
import { Button } from '../../ui/button';
import { toastSuccess, toastError } from '../../../toasts';
import { saveRecipe } from '../../../recipe/recipe_management';
import { Recipe } from '../../../recipe';
import { SubRecipeFormData } from './recipeFormSchema';

interface CreateSubRecipeInlineProps {
  isOpen: boolean;
  onClose: () => void;
  onSubRecipeSaved: (subRecipe: SubRecipeFormData) => void;
}

export default function CreateSubRecipeInline({
  isOpen,
  onClose,
  onSubRecipeSaved,
}: CreateSubRecipeInlineProps) {
  const [name, setName] = useState('');
  const [title, setTitle] = useState('');
  const [recipeDescription, setRecipeDescription] = useState('');
  const [instructions, setInstructions] = useState('');
  const [description, setDescription] = useState('');
  const [sequentialWhenRepeated, setSequentialWhenRepeated] = useState(false);
  const [values, setValues] = useState<Record<string, string>>({});
  const [newValueKey, setNewValueKey] = useState('');
  const [newValueValue, setNewValueValue] = useState('');
  const [isSaving, setIsSaving] = useState(false);

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

  const handleSave = async () => {
    if (!name.trim() || !title.trim() || !recipeDescription.trim() || !instructions.trim()) {
      toastError({
        title: 'Validation Failed',
        msg: 'Name, title, recipe description, and instructions are required.',
      });
      return;
    }

    setIsSaving(true);
    try {
      const recipe: Recipe = {
        version: '1.0.0',
        title: title.trim(),
        description: recipeDescription.trim(),
        instructions: instructions.trim(),
      };

      const savedRecipeId = await saveRecipe(recipe, null);

      const subRecipe: SubRecipeFormData = {
        name: name.trim(),
        path: `${savedRecipeId}.yaml`,
        description: description.trim() || undefined,
        sequential_when_repeated: sequentialWhenRepeated,
        values: Object.keys(values).length > 0 ? values : undefined,
      };

      toastSuccess({
        title: title.trim(),
        msg: 'Subrecipe created successfully',
      });

      onSubRecipeSaved(subRecipe);
      onClose();

      setName('');
      setTitle('');
      setRecipeDescription('');
      setInstructions('');
      setDescription('');
      setSequentialWhenRepeated(false);
      setValues({});
    } catch (error) {
      console.error('Failed to save subrecipe:', error);

      toastError({
        title: 'Save Failed',
        msg: `Failed to save subrecipe: ${error instanceof Error ? error.message : 'Unknown error'}`,
      });
    } finally {
      setIsSaving(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-[400] flex items-center justify-center bg-black/50">
      <div className="bg-background-default border border-borderSubtle rounded-lg w-[90vw] max-w-2xl max-h-[90vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b border-borderSubtle">
          <div>
            <h2 className="text-xl font-medium text-textProminent">Create New Subrecipe</h2>
            <p className="text-textSubtle text-sm">
              Create a simple recipe to use as a callable tool in your main recipe
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

          {/* Title Field */}
          <div>
            <label
              htmlFor="subrecipe-title"
              className="block text-sm font-medium text-text-standard mb-2"
            >
              Recipe Title <span className="text-red-500">*</span>
            </label>
            <input
              id="subrecipe-title"
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              className="w-full p-3 border border-border-subtle rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500"
              placeholder="e.g., Security Analysis Tool"
            />
          </div>

          {/* Recipe Description Field */}
          <div>
            <label
              htmlFor="recipe-description"
              className="block text-sm font-medium text-text-standard mb-2"
            >
              Recipe Description <span className="text-red-500">*</span>
            </label>
            <input
              id="recipe-description"
              type="text"
              value={recipeDescription}
              onChange={(e) => setRecipeDescription(e.target.value)}
              className="w-full p-3 border border-border-subtle rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500"
              placeholder="What this recipe does when executed"
            />
          </div>

          {/* Instructions Field */}
          <div>
            <label
              htmlFor="subrecipe-instructions"
              className="block text-sm font-medium text-text-standard mb-2"
            >
              Instructions <span className="text-red-500">*</span>
            </label>
            <textarea
              id="subrecipe-instructions"
              value={instructions}
              onChange={(e) => setInstructions(e.target.value)}
              className="w-full p-3 border border-border-subtle rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none font-mono text-sm"
              placeholder="Instructions for the AI when this subrecipe is called..."
              rows={8}
            />
          </div>

          {/* Tool Description Field */}
          <div>
            <label
              htmlFor="tool-description"
              className="block text-sm font-medium text-text-standard mb-2"
            >
              Tool Description
            </label>
            <textarea
              id="tool-description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              className="w-full p-3 border border-border-subtle rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
              placeholder="Optional description shown when this is called as a tool"
              rows={2}
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
              (Forces sequential execution of multiple instances)
            </span>
          </div>

          {/* Pre-configured Values */}
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
        <div className="flex gap-3 p-6 border-t border-borderSubtle justify-end">
          <Button onClick={onClose} variant="outline">
            Cancel
          </Button>
          <Button
            onClick={handleSave}
            disabled={
              !name.trim() ||
              !title.trim() ||
              !recipeDescription.trim() ||
              !instructions.trim() ||
              isSaving
            }
            className="bg-blue-500 hover:bg-blue-600 text-white inline-flex items-center gap-2"
          >
            {isSaving ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                Creating...
              </>
            ) : (
              <>
                <Save className="w-4 h-4" />
                Create & Add Subrecipe
              </>
            )}
          </Button>
        </div>
      </div>
    </div>
  );
}
