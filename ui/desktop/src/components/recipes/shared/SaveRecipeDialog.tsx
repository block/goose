import React, { useState } from 'react';
import { Button } from '../../ui/button';
import { Recipe } from '../../../recipe';
import { saveRecipe, generateRecipeFilename } from '../../../recipe/recipeStorage';
import { toastSuccess, toastError } from '../../../toasts';
import { useEscapeKey } from '../../../hooks/useEscapeKey';
import { Play } from 'lucide-react';

interface SaveRecipeDialogProps {
  isOpen: boolean;
  onClose: (wasSaved?: boolean) => void;
  onSuccess?: () => void;
  recipe: Recipe;
  suggestedName?: string;
  showSaveAndRun?: boolean;
  onSaveAndRun?: (recipe: Recipe) => void;
}

export default function SaveRecipeDialog({
  isOpen,
  onClose,
  onSuccess,
  recipe,
  suggestedName,
  showSaveAndRun = false,
  onSaveAndRun,
}: SaveRecipeDialogProps) {
  const [saveRecipeName, setSaveRecipeName] = useState(
    suggestedName || generateRecipeFilename(recipe)
  );
  const [saveGlobal, setSaveGlobal] = useState(true);
  const [saving, setSaving] = useState(false);

  useEscapeKey(isOpen, onClose);

  React.useEffect(() => {
    if (isOpen) {
      setSaveRecipeName(suggestedName || generateRecipeFilename(recipe));
      setSaveGlobal(true);
      setSaving(false);
    }
  }, [isOpen, suggestedName, recipe]);

  const handleSaveRecipe = async () => {
    if (!saveRecipeName.trim()) {
      return;
    }

    setSaving(true);
    try {
      if (!recipe.title || !recipe.description || !recipe.instructions) {
        throw new Error('Invalid recipe configuration: missing required fields');
      }

      await saveRecipe(recipe, {
        name: saveRecipeName.trim(),
        global: saveGlobal,
      });

      setSaveRecipeName('');
      onClose(true);

      toastSuccess({
        title: saveRecipeName.trim(),
        msg: 'Recipe saved successfully',
      });

      onSuccess?.();
    } catch (error) {
      console.error('Failed to save recipe:', error);

      toastError({
        title: 'Save Failed',
        msg: `Failed to save recipe: ${error instanceof Error ? error.message : 'Unknown error'}`,
        traceback: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setSaving(false);
    }
  };

  const handleSaveAndRunRecipe = async () => {
    if (!saveRecipeName.trim()) {
      return;
    }

    setSaving(true);
    try {
      if (!recipe.title || !recipe.description || !recipe.instructions) {
        throw new Error('Invalid recipe configuration: missing required fields');
      }

      await saveRecipe(recipe, {
        name: saveRecipeName.trim(),
        global: saveGlobal,
      });

      setSaveRecipeName('');
      onClose(true);

      toastSuccess({
        title: saveRecipeName.trim(),
        msg: 'Recipe saved and launched successfully',
      });

      // Launch the recipe in a new window
      onSaveAndRun?.(recipe);
      onSuccess?.();
    } catch (error) {
      console.error('Failed to save and run recipe:', error);

      toastError({
        title: 'Save and Run Failed',
        msg: `Failed to save and run recipe: ${error instanceof Error ? error.message : 'Unknown error'}`,
        traceback: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setSaving(false);
    }
  };

  const handleClose = () => {
    setSaveRecipeName('');
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-[500] flex items-center justify-center bg-black/50">
      <div className="bg-background-default border border-border-subtle rounded-lg p-6 w-96 max-w-[90vw]">
        <h3 className="text-lg font-medium text-text-standard mb-4">Save Recipe</h3>

        <div className="space-y-4">
          <div>
            <label
              htmlFor="recipe-name"
              className="block text-sm font-medium text-text-standard mb-2"
            >
              Recipe Name
            </label>
            <input
              id="recipe-name"
              type="text"
              value={saveRecipeName}
              onChange={(e) => setSaveRecipeName(e.target.value)}
              className="w-full p-3 border border-border-subtle rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500"
              placeholder="Enter recipe name"
              autoFocus
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-text-standard mb-2">
              Save Location
            </label>
            <div className="space-y-2">
              <label className="flex items-center">
                <input
                  type="radio"
                  name="save-location"
                  checked={saveGlobal}
                  onChange={() => setSaveGlobal(true)}
                  className="mr-2"
                />
                <span className="text-sm text-text-standard">
                  Global - Available across all Goose sessions
                </span>
              </label>
              <label className="flex items-center">
                <input
                  type="radio"
                  name="save-location"
                  checked={!saveGlobal}
                  onChange={() => setSaveGlobal(false)}
                  className="mr-2"
                />
                <span className="text-sm text-text-standard">
                  Directory - Available in the working directory
                </span>
              </label>
            </div>
          </div>
        </div>

        <div className="flex justify-end space-x-3 mt-6">
          <Button type="button" onClick={handleClose} variant="ghost" disabled={saving}>
            Cancel
          </Button>
          <Button
            onClick={handleSaveRecipe}
            disabled={!saveRecipeName.trim() || saving}
            variant="outline"
          >
            {saving ? 'Saving...' : 'Save Recipe'}
          </Button>
          {showSaveAndRun && (
            <Button
              onClick={handleSaveAndRunRecipe}
              disabled={!saveRecipeName.trim() || saving}
              variant="default"
              className="inline-flex items-center justify-center gap-2"
            >
              <Play className="w-4 h-4" />
              {saving ? 'Saving...' : 'Save & Run Recipe'}
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
