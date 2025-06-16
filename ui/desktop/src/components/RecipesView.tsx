import { useState, useEffect } from 'react';
import { listSavedRecipes, archiveRecipe, SavedRecipe } from '../recipe/recipeStorage';
import { FileText, Trash2, Bot, Calendar, Globe, Folder } from 'lucide-react';
import { ScrollArea } from './ui/scroll-area';
import BackButton from './ui/BackButton';
import MoreMenuLayout from './more_menu/MoreMenuLayout';

interface RecipesViewProps {
  onBack: () => void;
}

export default function RecipesView({ onBack }: RecipesViewProps) {
  const [savedRecipes, setSavedRecipes] = useState<SavedRecipe[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedRecipe, setSelectedRecipe] = useState<SavedRecipe | null>(null);
  const [showPreview, setShowPreview] = useState(false);

  useEffect(() => {
    loadSavedRecipes();
  }, []);

  const loadSavedRecipes = async () => {
    try {
      setLoading(true);
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
      // Use the recipe directly - no need for manual mapping
      window.electron.createChatWindow(
        undefined, // query
        undefined, // dir
        undefined, // version
        undefined, // resumeSessionId
        savedRecipe.recipe, // recipe config
        undefined // view type
      );
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

  if (loading) {
    return (
      <div className="h-screen w-full animate-[fadein_200ms_ease-in_forwards]">
        <MoreMenuLayout showMenu={false} />
        <div className="flex flex-col items-center justify-center h-full">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-borderProminent"></div>
          <p className="mt-4 text-textSubtle">Loading recipes...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="h-screen w-full animate-[fadein_200ms_ease-in_forwards]">
        <MoreMenuLayout showMenu={false} />
        <div className="flex flex-col items-center justify-center h-full">
          <p className="text-red-500 mb-4">{error}</p>
          <button
            onClick={loadSavedRecipes}
            className="px-4 py-2 bg-borderProminent text-white rounded-lg hover:bg-opacity-90"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="h-screen w-full animate-[fadein_200ms_ease-in_forwards]">
      <MoreMenuLayout showMenu={false} />

      <ScrollArea className="h-full w-full">
        <div className="flex flex-col pb-24">
          <div className="px-8 pt-6 pb-4">
            <BackButton onClick={onBack} />
            <h1 className="text-3xl font-medium text-textStandard mt-1">Saved Recipes</h1>
          </div>

          {/* Content Area */}
          <div className="flex-1 pt-[20px]">
            {savedRecipes.length === 0 ? (
              <div className="flex flex-col items-center justify-center h-full text-center px-8">
                <FileText className="w-16 h-16 text-textSubtle mb-4" />
                <h3 className="text-lg font-medium text-textStandard mb-2">No saved recipes</h3>
                <p className="text-textSubtle">
                  Save a recipe from an active session to see it here.
                </p>
              </div>
            ) : (
              <div className="space-y-8 px-8">
                {savedRecipes.map((savedRecipe) => (
                  <section
                    key={`${savedRecipe.isGlobal ? 'global' : 'local'}-${savedRecipe.name}`}
                    className="border-b border-borderSubtle pb-8"
                  >
                    <div className="flex justify-between items-start mb-4">
                      <div className="flex-1">
                        <div className="flex items-center gap-2 mb-1">
                          <h3 className="text-xl font-medium text-textStandard">
                            {savedRecipe.recipe.title}
                          </h3>
                          {savedRecipe.isGlobal ? (
                            <Globe className="w-4 h-4 text-textSubtle" />
                          ) : (
                            <Folder className="w-4 h-4 text-textSubtle" />
                          )}
                        </div>
                        <p className="text-textSubtle mb-2">{savedRecipe.recipe.description}</p>
                        <div className="flex items-center text-xs text-textSubtle">
                          <Calendar className="w-3 h-3 mr-1" />
                          {savedRecipe.lastModified.toLocaleDateString()}
                        </div>
                      </div>
                    </div>

                    <div className="flex items-center gap-3">
                      <button
                        onClick={() => handleLoadRecipe(savedRecipe)}
                        className="flex items-center gap-2 px-4 py-2 bg-black dark:bg-white text-white dark:text-black rounded-lg hover:bg-opacity-90 transition-colors text-sm font-medium"
                      >
                        <Bot className="w-4 h-4" />
                        Use Recipe
                      </button>
                      <button
                        onClick={() => handlePreviewRecipe(savedRecipe)}
                        className="flex items-center gap-2 px-4 py-2 border border-borderSubtle rounded-lg hover:border-borderStandard transition-colors text-sm"
                      >
                        <FileText className="w-4 h-4" />
                        Preview
                      </button>
                      <button
                        onClick={() => handleDeleteRecipe(savedRecipe)}
                        className="flex items-center gap-2 px-4 py-2 text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg transition-colors text-sm"
                      >
                        <Trash2 className="w-4 h-4" />
                        Delete
                      </button>
                    </div>
                  </section>
                ))}
              </div>
            )}
          </div>
        </div>
      </ScrollArea>

      {/* Preview Modal */}
      {showPreview && selectedRecipe && (
        <div className="fixed inset-0 z-[300] flex items-center justify-center bg-black bg-opacity-50">
          <div className="bg-bgApp border border-borderSubtle rounded-lg p-6 w-[600px] max-w-[90vw] max-h-[80vh] overflow-y-auto">
            <div className="flex items-start justify-between mb-4">
              <div>
                <h3 className="text-xl font-medium text-textStandard">
                  {selectedRecipe.recipe.title}
                </h3>
                <p className="text-sm text-textSubtle">
                  {selectedRecipe.isGlobal ? 'Global recipe' : 'Project recipe'}
                </p>
              </div>
              <button
                onClick={() => setShowPreview(false)}
                className="text-textSubtle hover:text-textStandard text-2xl leading-none"
              >
                ×
              </button>
            </div>

            <div className="space-y-6">
              <div>
                <h4 className="text-sm font-medium text-textStandard mb-2">Description</h4>
                <p className="text-textSubtle">{selectedRecipe.recipe.description}</p>
              </div>

              {selectedRecipe.recipe.instructions && (
                <div>
                  <h4 className="text-sm font-medium text-textStandard mb-2">Instructions</h4>
                  <div className="bg-bgSubtle border border-borderSubtle p-3 rounded-lg">
                    <pre className="text-sm text-textSubtle whitespace-pre-wrap font-mono">
                      {selectedRecipe.recipe.instructions}
                    </pre>
                  </div>
                </div>
              )}

              {selectedRecipe.recipe.prompt && (
                <div>
                  <h4 className="text-sm font-medium text-textStandard mb-2">Initial Prompt</h4>
                  <div className="bg-bgSubtle border border-borderSubtle p-3 rounded-lg">
                    <pre className="text-sm text-textSubtle whitespace-pre-wrap font-mono">
                      {selectedRecipe.recipe.prompt}
                    </pre>
                  </div>
                </div>
              )}

              {selectedRecipe.recipe.activities && selectedRecipe.recipe.activities.length > 0 && (
                <div>
                  <h4 className="text-sm font-medium text-textStandard mb-2">Activities</h4>
                  <div className="flex flex-wrap gap-2">
                    {selectedRecipe.recipe.activities.map((activity, index) => (
                      <span
                        key={index}
                        className="px-2 py-1 bg-bgSubtle border border-borderSubtle text-textSubtle rounded text-sm"
                      >
                        {activity}
                      </span>
                    ))}
                  </div>
                </div>
              )}
            </div>

            <div className="flex justify-end gap-3 mt-6 pt-4 border-t border-borderSubtle">
              <button
                onClick={() => setShowPreview(false)}
                className="px-4 py-2 text-textSubtle hover:text-textStandard transition-colors"
              >
                Close
              </button>
              <button
                onClick={() => {
                  setShowPreview(false);
                  handleLoadRecipe(selectedRecipe);
                }}
                className="px-4 py-2 bg-black dark:bg-white text-white dark:text-black rounded-lg hover:bg-opacity-90 transition-colors font-medium"
              >
                Load Recipe
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
