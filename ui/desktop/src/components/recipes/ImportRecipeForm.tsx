import { useState } from 'react';
import { Download } from 'lucide-react';
import { Button } from '../ui/button';
import { Recipe, decodeRecipe } from '../../recipe';
import { saveRecipe, generateRecipeFilename } from '../../recipe/recipeStorage';
import { toastSuccess, toastError } from '../../toasts';
import { useEscapeKey } from '../../hooks/useEscapeKey';

interface ImportRecipeFormProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
}

export default function ImportRecipeForm({ isOpen, onClose, onSuccess }: ImportRecipeFormProps) {
  const [importDeeplink, setImportDeeplink] = useState('');
  const [importRecipeName, setImportRecipeName] = useState('');
  const [importGlobal, setImportGlobal] = useState(true);
  const [importing, setImporting] = useState(false);

  // Handle Esc key for modal
  useEscapeKey(isOpen, () => {
    onClose();
    setImportDeeplink('');
    setImportRecipeName('');
  });

  // Function to parse deeplink and extract recipe
  const parseDeeplink = async (deeplink: string): Promise<Recipe | null> => {
    try {
      const cleanLink = deeplink.trim();

      if (!cleanLink.startsWith('goose://recipe?config=')) {
        throw new Error('Invalid deeplink format. Expected: goose://recipe?config=...');
      }

      const recipeEncoded = cleanLink.replace('goose://recipe?config=', '');

      if (!recipeEncoded) {
        throw new Error('No recipe configuration found in deeplink');
      }
      const recipe = await decodeRecipe(recipeEncoded);

      if (!recipe.title || !recipe.description) {
        throw new Error('Recipe is missing required fields (title, description)');
      }

      if (!recipe.instructions && !recipe.prompt) {
        throw new Error('Recipe must have either instructions or prompt');
      }

      return recipe;
    } catch (error) {
      console.error('Failed to parse deeplink:', error);
      return null;
    }
  };

  const handleImportRecipe = async () => {
    if (!importDeeplink.trim() || !importRecipeName.trim()) {
      return;
    }

    setImporting(true);
    try {
      const recipe = await parseDeeplink(importDeeplink.trim());

      if (!recipe) {
        throw new Error('Invalid deeplink or recipe format');
      }

      await saveRecipe(recipe, {
        name: importRecipeName.trim(),
        global: importGlobal,
      });

      // Reset dialog state
      onClose();
      setImportDeeplink('');
      setImportRecipeName('');

      onSuccess();

      toastSuccess({
        title: importRecipeName.trim(),
        msg: 'Recipe imported successfully',
      });
    } catch (error) {
      console.error('Failed to import recipe:', error);

      toastError({
        title: 'Import Failed',
        msg: `Failed to import recipe: ${error instanceof Error ? error.message : 'Unknown error'}`,
        traceback: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setImporting(false);
    }
  };

  const handleClose = () => {
    onClose();
    setImportDeeplink('');
    setImportRecipeName('');
  };

  // Auto-generate recipe name when deeplink changes
  const handleDeeplinkChange = async (value: string) => {
    setImportDeeplink(value);

    if (value.trim()) {
      try {
        const recipe = await parseDeeplink(value.trim());
        if (recipe && recipe.title) {
          const suggestedName = generateRecipeFilename(recipe);
          setImportRecipeName(suggestedName);
        }
      } catch (error) {
        // Silently handle parsing errors during auto-suggest
        console.log('Could not parse deeplink for auto-suggest:', error);
      }
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-[300] flex items-center justify-center bg-black/50">
      <div className="bg-background-default border border-border-subtle rounded-lg p-6 w-[500px] max-w-[90vw]">
        <h3 className="text-lg font-medium text-text-standard mb-4">Import Recipe</h3>

        <div className="space-y-4">
          <div>
            <label
              htmlFor="import-deeplink"
              className="block text-sm font-medium text-text-standard mb-2"
            >
              Recipe Deeplink
            </label>
            <textarea
              id="import-deeplink"
              value={importDeeplink}
              onChange={(e) => handleDeeplinkChange(e.target.value)}
              className="w-full p-3 border border-border-subtle rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
              placeholder="Paste your goose://recipe?config=... deeplink here"
              rows={3}
              autoFocus
            />
            <p className="text-xs text-text-muted mt-1">
              Paste a recipe deeplink starting with "goose://recipe?config="
            </p>
          </div>

          <div>
            <label
              htmlFor="import-recipe-name"
              className="block text-sm font-medium text-text-standard mb-2"
            >
              Recipe Name
            </label>
            <input
              id="import-recipe-name"
              type="text"
              value={importRecipeName}
              onChange={(e) => setImportRecipeName(e.target.value)}
              className="w-full p-3 border border-border-subtle rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500"
              placeholder="Enter a name for the imported recipe"
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
                  name="import-save-location"
                  checked={importGlobal}
                  onChange={() => setImportGlobal(true)}
                  className="mr-2"
                />
                <span className="text-sm text-text-standard">
                  Global - Available across all Goose sessions
                </span>
              </label>
              <label className="flex items-center">
                <input
                  type="radio"
                  name="import-save-location"
                  checked={!importGlobal}
                  onChange={() => setImportGlobal(false)}
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
          <Button onClick={handleClose} variant="ghost" disabled={importing}>
            Cancel
          </Button>
          <Button
            onClick={handleImportRecipe}
            disabled={!importDeeplink.trim() || !importRecipeName.trim() || importing}
            variant="default"
          >
            {importing ? 'Importing...' : 'Import Recipe'}
          </Button>
        </div>
      </div>
    </div>
  );
}

// Export the button component for easy access
export function ImportRecipeButton({ onClick }: { onClick: () => void }) {
  return (
    <Button onClick={onClick} variant="default" size="sm" className="flex items-center gap-2">
      <Download className="w-4 h-4" />
      Import Recipe
    </Button>
  );
}
