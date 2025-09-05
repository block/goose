import React, { useState, useEffect, useRef } from 'react';
import { FileText, ChevronDown, FolderOpen } from 'lucide-react';
import { Button } from './ui/button';
import { Recipe } from '../recipe';
import { toastError } from '../toasts';
import * as yaml from 'yaml';
import { RecipeParameterModal } from './RecipeParameterModal';

interface RecipeSelectorProps {
  onSelectRecipe: (recipe: Recipe) => void;
  className?: string;
}

interface LocalRecipe {
  filename: string;
  recipe: Recipe;
}

export const RecipeSelector: React.FC<RecipeSelectorProps> = ({
  onSelectRecipe,
  className = '',
}) => {
  const [recipes, setRecipes] = useState<LocalRecipe[]>([]);
  const [isOpen, setIsOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [selectedRecipe, setSelectedRecipe] = useState<Recipe | null>(null);
  const [showParameterModal, setShowParameterModal] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const [recipePath, setRecipePath] = useState<string>('');

  // Load recipes when component mounts
  useEffect(() => {
    loadLocalRecipes();
  }, []);

  // Close dropdown when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };

    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside);
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [isOpen]);

  const loadLocalRecipes = async () => {
    setLoading(true);
    try {
      // Get the current working directory
      const config = window.electron.getConfig();
      const workingDir = config.GOOSE_WORKING_DIR || process.cwd();
      const recipeDir = `${workingDir}/.goose/recipes`;
      setRecipePath(recipeDir);

      // Check if the recipe directory exists
      const dirExists = await window.electron.ensureDirectory(recipeDir);
      if (!dirExists) {
        setRecipes([]);
        setLoading(false);
        return;
      }

      // List all YAML files in the recipe directory
      const yamlFiles = await window.electron.listFiles(recipeDir, '.yaml');
      const ymlFiles = await window.electron.listFiles(recipeDir, '.yml');
      const allFiles = [...yamlFiles, ...ymlFiles];

      const loadedRecipes: LocalRecipe[] = [];

      // Read and parse each recipe file
      for (const file of allFiles) {
        try {
          const filePath = `${recipeDir}/${file}`;
          const fileResponse = await window.electron.readFile(filePath);

          if (fileResponse.found && fileResponse.file) {
            // Parse the YAML content
            const recipeData = yaml.parse(fileResponse.file) as Recipe;

            // Validate that it has required fields
            if (recipeData && recipeData.title && recipeData.description) {
              loadedRecipes.push({
                filename: file,
                recipe: recipeData,
              });
            }
          }
        } catch (error) {
          console.error(`Failed to load recipe ${file}:`, error);
        }
      }

      // Sort recipes by title
      loadedRecipes.sort((a, b) => a.recipe.title.localeCompare(b.recipe.title));
      setRecipes(loadedRecipes);
    } catch (error) {
      console.error('Failed to load recipes:', error);
      toastError({
        title: 'Failed to load recipes',
        msg: 'Could not load recipes from .goose/recipes directory',
      });
    } finally {
      setLoading(false);
    }
  };

  // Don't render anything if there are no recipes
  if (recipes.length === 0 && !loading) {
    return null;
  }

  const handleSelectRecipe = (localRecipe: LocalRecipe) => {
    // Check if recipe has parameters
    if (localRecipe.recipe.parameters && localRecipe.recipe.parameters.length > 0) {
      setSelectedRecipe(localRecipe.recipe);
      setShowParameterModal(true);
      setIsOpen(false);
    } else {
      onSelectRecipe(localRecipe.recipe);
      setIsOpen(false);
    }
  };

  const handleParameterSubmit = (values: Record<string, string>) => {
    if (!selectedRecipe) return;

    // Replace parameters in the prompt
    let processedPrompt = selectedRecipe.prompt || '';

    // Replace {{key}} with the actual values
    Object.entries(values).forEach(([key, value]) => {
      const regex = new RegExp(`{{\\s*${key}\\s*}}`, 'g');
      processedPrompt = processedPrompt.replace(regex, value);
    });

    // Create a new recipe with the processed prompt
    const processedRecipe = {
      ...selectedRecipe,
      prompt: processedPrompt,
      // Clear parameters since they've been processed
      parameters: undefined,
    };

    onSelectRecipe(processedRecipe as Recipe);
    setSelectedRecipe(null);
    setShowParameterModal(false);
  };

  const handleOpenRecipeFolder = async () => {
    if (recipePath) {
      await window.electron.openDirectoryInExplorer(recipePath);
    }
  };

  return (
    <div className={`relative ${className}`} ref={dropdownRef}>
      <Button
        type="button"
        onClick={() => setIsOpen(!isOpen)}
        variant="ghost"
        size="sm"
        className="flex items-center gap-1 text-text-default/70 hover:text-text-default text-xs cursor-pointer transition-colors"
        disabled={loading}
      >
        <FileText className="w-4 h-4" />
        <span>Run Recipe</span>
        <ChevronDown className={`w-3 h-3 transition-transform ${isOpen ? 'rotate-180' : ''}`} />
      </Button>

      {isOpen && (
        <div className="absolute bottom-full left-0 mb-2 w-64 max-h-80 overflow-y-auto bg-background-default border border-borderStandard rounded-lg shadow-lg z-50">
          {loading ? (
            <div className="p-4 text-center text-textSubtle text-sm">Loading recipes...</div>
          ) : recipes.length === 0 ? (
            <div className="p-4 text-center text-textSubtle text-sm">
              No recipes found in .goose/recipes
            </div>
          ) : (
            <div className="py-2">
              {recipes.map((localRecipe, index) => (
                <button
                  key={`${localRecipe.filename}-${index}`}
                  onClick={() => handleSelectRecipe(localRecipe)}
                  className="w-full px-4 py-2 text-left hover:bg-background-medium transition-colors"
                >
                  <div className="font-medium text-sm text-textStandard">
                    {localRecipe.recipe.title}
                  </div>
                  <div className="text-xs text-textSubtle mt-0.5 line-clamp-2">
                    {localRecipe.recipe.description}
                  </div>
                  <div className="text-xs text-textSubtle/60 mt-1">{localRecipe.filename}</div>
                </button>
              ))}
              <div className="border-t border-borderSubtle mt-2 pt-2 px-2 pb-2">
                <button
                  onClick={handleOpenRecipeFolder}
                  className="w-full flex items-center gap-2 px-2 py-1.5 text-xs text-textSubtle hover:text-textStandard hover:bg-background-medium rounded transition-colors"
                >
                  <FolderOpen className="w-3.5 h-3.5" />
                  <span>Open Recipe Folder</span>
                </button>
              </div>
            </div>
          )}
        </div>
      )}

      {selectedRecipe && (
        <RecipeParameterModal
          recipe={selectedRecipe}
          isOpen={showParameterModal}
          onClose={() => {
            setShowParameterModal(false);
            setSelectedRecipe(null);
          }}
          onSubmit={handleParameterSubmit}
        />
      )}
    </div>
  );
};
