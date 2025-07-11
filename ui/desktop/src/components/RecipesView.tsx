import { useState, useEffect } from 'react';
import {
  listSavedRecipes,
  archiveRecipe,
  SavedRecipe,
  saveRecipe,
  generateRecipeFilename,
} from '../recipe/recipeStorage';
import {
  FileText,
  Trash2,
  Bot,
  Calendar,
  Globe,
  Folder,
  AlertCircle,
  Download,
} from 'lucide-react';
import { ScrollArea } from './ui/scroll-area';
import { Card } from './ui/card';
import { Button } from './ui/button';
import { Skeleton } from './ui/skeleton';
import { MainPanelLayout } from './Layout/MainPanelLayout';
import { Recipe } from '../recipe';
import { Buffer } from 'buffer';
import { toastSuccess, toastError } from '../toasts';

interface RecipesViewProps {
  onLoadRecipe?: (recipe: Recipe) => void;
}

export default function RecipesView({ onLoadRecipe }: RecipesViewProps = {}) {
  const [savedRecipes, setSavedRecipes] = useState<SavedRecipe[]>([]);
  const [loading, setLoading] = useState(true);
  const [showSkeleton, setShowSkeleton] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedRecipe, setSelectedRecipe] = useState<SavedRecipe | null>(null);
  const [showPreview, setShowPreview] = useState(false);
  const [showContent, setShowContent] = useState(false);
  const [showImportDialog, setShowImportDialog] = useState(false);
  const [importDeeplink, setImportDeeplink] = useState('');
  const [importRecipeName, setImportRecipeName] = useState('');
  const [importGlobal, setImportGlobal] = useState(true);
  const [importing, setImporting] = useState(false);

  useEffect(() => {
    loadSavedRecipes();
  }, []);

  // Minimum loading time to prevent skeleton flash
  useEffect(() => {
    if (!loading && showSkeleton) {
      const timer = setTimeout(() => {
        setShowSkeleton(false);
        // Add a small delay before showing content for fade-in effect
        setTimeout(() => {
          setShowContent(true);
        }, 50);
      }, 300); // Show skeleton for at least 300ms

      // eslint-disable-next-line no-undef
      return () => clearTimeout(timer);
    }
    return () => void 0;
  }, [loading, showSkeleton]);

  const loadSavedRecipes = async () => {
    try {
      setLoading(true);
      setShowSkeleton(true);
      setShowContent(false);
      setError(null);
      const recipes = await listSavedRecipes();
      setSavedRecipes(recipes);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load recipes');
      console.error('Failed to load saved recipes:', err);
    } finally {
      setLoading(false);
    }
  };

  const handleLoadRecipe = async (savedRecipe: SavedRecipe) => {
    try {
      if (onLoadRecipe) {
        // Use the callback to navigate within the same window
        onLoadRecipe(savedRecipe.recipe);
      } else {
        // Fallback to creating a new window (for backwards compatibility)
        window.electron.createChatWindow(
          undefined, // query
          undefined, // dir
          undefined, // version
          undefined, // resumeSessionId
          savedRecipe.recipe, // recipe config
          undefined // view type
        );
      }
    } catch (err) {
      console.error('Failed to load recipe:', err);
      setError(err instanceof Error ? err.message : 'Failed to load recipe');
    }
  };

  const handleDeleteRecipe = async (savedRecipe: SavedRecipe) => {
    // TODO: Use Electron's dialog API for confirmation
    const result = await window.electron.showMessageBox({
      type: 'warning',
      buttons: ['Cancel', 'Delete'],
      defaultId: 0,
      title: 'Delete Recipe',
      message: `Are you sure you want to delete "${savedRecipe.name}"?`,
      detail: 'Deleted recipes can be restored later.',
    });

    if (result.response !== 1) {
      return;
    }

    try {
      await archiveRecipe(savedRecipe.name, savedRecipe.isGlobal);
      // Reload the recipes list
      await loadSavedRecipes();
    } catch (err) {
      console.error('Failed to archive recipe:', err);
      setError(err instanceof Error ? err.message : 'Failed to archive recipe');
    }
  };

  const handlePreviewRecipe = (savedRecipe: SavedRecipe) => {
    setSelectedRecipe(savedRecipe);
    setShowPreview(true);
  };

  // Function to parse deeplink and extract recipe
  const parseDeeplink = (deeplink: string): Recipe | null => {
    try {
      const cleanLink = deeplink.trim();

      if (!cleanLink.startsWith('goose://recipe?config=')) {
        throw new Error('Invalid deeplink format. Expected: goose://recipe?config=...');
      }

      // Extract and decode the base64 config
      const configBase64 = cleanLink.replace('goose://recipe?config=', '');

      if (!configBase64) {
        throw new Error('No recipe configuration found in deeplink');
      }
      const configJson = Buffer.from(configBase64, 'base64').toString('utf-8');
      const recipe = JSON.parse(configJson) as Recipe;

      if (!recipe.title || !recipe.description || !recipe.instructions) {
        throw new Error('Recipe is missing required fields (title, description, instructions)');
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
      const recipe = parseDeeplink(importDeeplink.trim());

      if (!recipe) {
        throw new Error('Invalid deeplink or recipe format');
      }

      await saveRecipe(recipe, {
        name: importRecipeName.trim(),
        global: importGlobal,
      });

      // Reset dialog state
      setShowImportDialog(false);
      setImportDeeplink('');
      setImportRecipeName('');

      await loadSavedRecipes();

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

  const handleImportClick = () => {
    setImportDeeplink('');
    setImportRecipeName('');
    setImportGlobal(true);
    setShowImportDialog(true);
  };

  // Auto-generate recipe name when deeplink changes
  const handleDeeplinkChange = (value: string) => {
    setImportDeeplink(value);

    if (value.trim()) {
      const recipe = parseDeeplink(value.trim());
      if (recipe && recipe.title) {
        const suggestedName = generateRecipeFilename(recipe);
        setImportRecipeName(suggestedName);
      }
    }
  };

  // Render a recipe item
  const RecipeItem = ({ savedRecipe }: { savedRecipe: SavedRecipe }) => (
    <Card className="py-2 px-4 mb-2 bg-background-default border-none hover:bg-background-muted cursor-pointer transition-all duration-150">
      <div className="flex justify-between items-start gap-4">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2 mb-1">
            <h3 className="text-base truncate max-w-[50vw]">{savedRecipe.recipe.title}</h3>
            {savedRecipe.isGlobal ? (
              <Globe className="w-4 h-4 text-text-muted flex-shrink-0" />
            ) : (
              <Folder className="w-4 h-4 text-text-muted flex-shrink-0" />
            )}
          </div>
          <p className="text-text-muted text-sm mb-2 line-clamp-2">
            {savedRecipe.recipe.description}
          </p>
          <div className="flex items-center text-xs text-text-muted">
            <Calendar className="w-3 h-3 mr-1" />
            {savedRecipe.lastModified.toLocaleDateString()}
          </div>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          <Button
            onClick={(e) => {
              e.stopPropagation();
              handleLoadRecipe(savedRecipe);
            }}
            size="sm"
            className="h-8"
          >
            <Bot className="w-4 h-4 mr-1" />
            Use
          </Button>
          <Button
            onClick={(e) => {
              e.stopPropagation();
              handlePreviewRecipe(savedRecipe);
            }}
            variant="outline"
            size="sm"
            className="h-8"
          >
            <FileText className="w-4 h-4 mr-1" />
            Preview
          </Button>
          <Button
            onClick={(e) => {
              e.stopPropagation();
              handleDeleteRecipe(savedRecipe);
            }}
            variant="ghost"
            size="sm"
            className="h-8 text-red-500 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20"
          >
            <Trash2 className="w-4 h-4" />
          </Button>
        </div>
      </div>
    </Card>
  );

  // Render skeleton loader for recipe items
  const RecipeSkeleton = () => (
    <Card className="p-2 mb-2 bg-background-default">
      <div className="flex justify-between items-start gap-4">
        <div className="min-w-0 flex-1">
          <Skeleton className="h-5 w-3/4 mb-2" />
          <Skeleton className="h-4 w-full mb-2" />
          <Skeleton className="h-4 w-24" />
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <Skeleton className="h-8 w-16" />
          <Skeleton className="h-8 w-20" />
          <Skeleton className="h-8 w-8" />
        </div>
      </div>
    </Card>
  );

  const renderContent = () => {
    if (loading || showSkeleton) {
      return (
        <div className="space-y-6">
          <div className="space-y-3">
            <Skeleton className="h-6 w-24" />
            <div className="space-y-2">
              <RecipeSkeleton />
              <RecipeSkeleton />
              <RecipeSkeleton />
            </div>
          </div>
        </div>
      );
    }

    if (error) {
      return (
        <div className="flex flex-col items-center justify-center h-full text-text-muted">
          <AlertCircle className="h-12 w-12 text-red-500 mb-4" />
          <p className="text-lg mb-2">Error Loading Recipes</p>
          <p className="text-sm text-center mb-4">{error}</p>
          <Button onClick={loadSavedRecipes} variant="default">
            Try Again
          </Button>
        </div>
      );
    }

    if (savedRecipes.length === 0) {
      return (
        <div className="flex flex-col justify-center pt-2 h-full">
          <p className="text-lg">No saved recipes</p>
          <p className="text-sm text-text-muted">Recipe saved from chats will show up here.</p>
        </div>
      );
    }

    return (
      <div className="space-y-2">
        {savedRecipes.map((savedRecipe) => (
          <RecipeItem
            key={`${savedRecipe.isGlobal ? 'global' : 'local'}-${savedRecipe.name}`}
            savedRecipe={savedRecipe}
          />
        ))}
      </div>
    );
  };

  return (
    <>
      <MainPanelLayout>
        <div className="flex-1 flex flex-col min-h-0">
          <div className="bg-background-default px-8 pb-8 pt-16">
            <div className="flex flex-col animate-in fade-in duration-500">
              <div className="flex justify-between items-center mb-1">
                <h1 className="text-4xl font-light">Recipes</h1>
                <Button
                  onClick={handleImportClick}
                  variant="default"
                  size="sm"
                  className="flex items-center gap-2"
                >
                  <Download className="w-4 h-4" />
                  Import Recipe
                </Button>
              </div>
              <p className="text-sm text-text-muted mb-1">
                View and manage your saved recipes to quickly start new sessions with predefined
                configurations.
              </p>
            </div>
          </div>

          <div className="flex-1 min-h-0 relative px-8">
            <ScrollArea className="h-full">
              <div
                className={`h-full relative transition-all duration-300 ${
                  showContent ? 'opacity-100 animate-in fade-in ' : 'opacity-0'
                }`}
              >
                {renderContent()}
              </div>
            </ScrollArea>
          </div>
        </div>
      </MainPanelLayout>

      {/* Preview Modal */}
      {showPreview && selectedRecipe && (
        <div className="fixed inset-0 z-[300] flex items-center justify-center bg-black/50">
          <div className="bg-background-default border border-border-subtle rounded-lg p-6 w-[600px] max-w-[90vw] max-h-[80vh] overflow-y-auto">
            <div className="flex items-start justify-between mb-4">
              <div>
                <h3 className="text-xl font-medium text-text-standard">
                  {selectedRecipe.recipe.title}
                </h3>
                <p className="text-sm text-text-muted">
                  {selectedRecipe.isGlobal ? 'Global recipe' : 'Project recipe'}
                </p>
              </div>
              <button
                onClick={() => setShowPreview(false)}
                className="text-text-muted hover:text-text-standard text-2xl leading-none"
              >
                ×
              </button>
            </div>

            <div className="space-y-6">
              <div>
                <h4 className="text-sm font-medium text-text-standard mb-2">Description</h4>
                <p className="text-text-muted">{selectedRecipe.recipe.description}</p>
              </div>

              {selectedRecipe.recipe.instructions && (
                <div>
                  <h4 className="text-sm font-medium text-text-standard mb-2">Instructions</h4>
                  <div className="bg-background-muted border border-border-subtle p-3 rounded-lg">
                    <pre className="text-sm text-text-muted whitespace-pre-wrap font-mono">
                      {selectedRecipe.recipe.instructions}
                    </pre>
                  </div>
                </div>
              )}

              {selectedRecipe.recipe.prompt && (
                <div>
                  <h4 className="text-sm font-medium text-text-standard mb-2">Initial Prompt</h4>
                  <div className="bg-background-muted border border-border-subtle p-3 rounded-lg">
                    <pre className="text-sm text-text-muted whitespace-pre-wrap font-mono">
                      {selectedRecipe.recipe.prompt}
                    </pre>
                  </div>
                </div>
              )}

              {selectedRecipe.recipe.activities && selectedRecipe.recipe.activities.length > 0 && (
                <div>
                  <h4 className="text-sm font-medium text-text-standard mb-2">Activities</h4>
                  <div className="flex flex-wrap gap-2">
                    {selectedRecipe.recipe.activities.map((activity, index) => (
                      <span
                        key={index}
                        className="px-2 py-1 bg-background-muted border border-border-subtle text-text-muted rounded text-sm"
                      >
                        {activity}
                      </span>
                    ))}
                  </div>
                </div>
              )}
            </div>

            <div className="flex justify-end gap-3 mt-6 pt-4 border-t border-border-subtle">
              <Button onClick={() => setShowPreview(false)} variant="ghost">
                Close
              </Button>
              <Button
                onClick={() => {
                  setShowPreview(false);
                  handleLoadRecipe(selectedRecipe);
                }}
                variant="default"
              >
                Load Recipe
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Import Recipe Dialog */}
      {showImportDialog && (
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
              <Button
                onClick={() => {
                  setShowImportDialog(false);
                  setImportDeeplink('');
                  setImportRecipeName('');
                }}
                variant="ghost"
                disabled={importing}
              >
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
      )}
    </>
  );
}
