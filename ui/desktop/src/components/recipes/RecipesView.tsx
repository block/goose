import { useState, useEffect, useMemo } from 'react';
import { listSavedRecipes, convertToLocaleDateString } from '../../recipe/recipeStorage';
import { FileText, Trash2, Bot, Calendar, AlertCircle } from 'lucide-react';
import { ScrollArea } from '../ui/scroll-area';
import { Card } from '../ui/card';
import { Button } from '../ui/button';
import { Skeleton } from '../ui/skeleton';
import { MainPanelLayout } from '../Layout/MainPanelLayout';
import { Recipe, generateDeepLink } from '../../recipe';
import { toastSuccess, toastError } from '../../toasts';
import { useEscapeKey } from '../../hooks/useEscapeKey';
import { deleteRecipe, RecipeManifestResponse } from '../../api';
import CreateRecipeForm, { CreateRecipeButton } from './CreateRecipeForm';
import ImportRecipeForm, { ImportRecipeButton } from './ImportRecipeForm';
import { filterValidUsedParameters } from '../../utils/providerUtils';

export default function RecipesView() {
  const [savedRecipes, setSavedRecipes] = useState<RecipeManifestResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const [showSkeleton, setShowSkeleton] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedRecipe, setSelectedRecipe] = useState<RecipeManifestResponse | null>(null);
  const [showPreview, setShowPreview] = useState(false);
  const [showContent, setShowContent] = useState(false);
  const [previewDeeplink, setPreviewDeeplink] = useState<string>('');

  // Form dialog states
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [showImportDialog, setShowImportDialog] = useState(false);

  useEffect(() => {
    loadSavedRecipes();
  }, []);

  // Handle Esc key for preview modal
  useEscapeKey(showPreview, () => setShowPreview(false));

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
      const recipeManifestResponses = await listSavedRecipes();
      setSavedRecipes(recipeManifestResponses);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load recipes');
      console.error('Failed to load saved recipes:', err);
    } finally {
      setLoading(false);
    }
  };

  const handleLoadRecipe = async (recipe: Recipe) => {
    try {
      // onLoadRecipe is not working for loading recipes. It looks correct
      // but the instructions are not flowing through to the server.
      // Needs a fix but commenting out to get prod back up and running.
      //
      // if (onLoadRecipe) {
      //   // Use the callback to navigate within the same window
      //   onLoadRecipe(savedRecipe.recipe);
      // } else {
      // Fallback to creating a new window (for backwards compatibility)
      window.electron.createChatWindow(
        undefined, // query
        undefined, // dir
        undefined, // version
        undefined, // resumeSessionId
        recipe, // recipe config
        undefined // view type
      );
      // }
    } catch (err) {
      console.error('Failed to load recipe:', err);
      setError(err instanceof Error ? err.message : 'Failed to load recipe');
    }
  };

  const handleDeleteRecipe = async (recipeManifest: RecipeManifestResponse) => {
    // TODO: Use Electron's dialog API for confirmation
    const result = await window.electron.showMessageBox({
      type: 'warning',
      buttons: ['Cancel', 'Delete'],
      defaultId: 0,
      title: 'Delete Recipe',
      message: `Are you sure you want to delete "${recipeManifest.name}"?`,
      detail: 'Recipe file will be deleted.',
    });

    if (result.response !== 1) {
      return;
    }

    try {
      await deleteRecipe({ body: { id: recipeManifest.id } });
      await loadSavedRecipes();
      toastSuccess({
        title: recipeManifest.name,
        msg: 'Recipe deleted successfully',
      });
    } catch (err) {
      console.error('Failed to delete recipe:', err);
      setError(err instanceof Error ? err.message : 'Failed to delete recipe');
    }
  };

  const handlePreviewRecipe = async (recipeManifest: RecipeManifestResponse) => {
    setSelectedRecipe(recipeManifest);
    setShowPreview(true);

    // Generate deeplink for preview
    try {
      const deeplink = await generateDeepLink(recipeManifest.recipe);
      setPreviewDeeplink(deeplink);
    } catch (error) {
      console.error('Failed to generate deeplink for preview:', error);
      setPreviewDeeplink('Error generating deeplink');
    }
  };

  const filteredPreviewParameters = useMemo(() => {
    if (!selectedRecipe?.recipe.parameters) {
      return [];
    }

    return filterValidUsedParameters(selectedRecipe.recipe.parameters, {
      instructions: selectedRecipe.recipe.instructions || undefined,
      prompt: selectedRecipe.recipe.prompt || undefined,
    });
  }, [
    selectedRecipe?.recipe.parameters,
    selectedRecipe?.recipe.instructions,
    selectedRecipe?.recipe.prompt,
  ]);

  // Render a recipe item
  const RecipeItem = ({
    recipeManifestResponse,
    recipeManifestResponse: { recipe, lastModified },
  }: {
    recipeManifestResponse: RecipeManifestResponse;
  }) => (
    <Card className="py-2 px-4 mb-2 bg-background-default border-none hover:bg-background-muted cursor-pointer transition-all duration-150">
      <div className="flex justify-between items-start gap-4">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2 mb-1">
            <h3 className="text-base truncate max-w-[50vw]">{recipe.title}</h3>
          </div>
          <p className="text-text-muted text-sm mb-2 line-clamp-2">{recipe.description}</p>
          <div className="flex items-center text-xs text-text-muted">
            <Calendar className="w-3 h-3 mr-1" />
            {convertToLocaleDateString(lastModified)}
          </div>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          <Button
            onClick={(e) => {
              e.stopPropagation();
              handleLoadRecipe(recipe);
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
              handlePreviewRecipe(recipeManifestResponse);
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
              handleDeleteRecipe(recipeManifestResponse);
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
        {savedRecipes.map((recipeManifestResponse: RecipeManifestResponse) => (
          <RecipeItem
            key={recipeManifestResponse.id}
            recipeManifestResponse={recipeManifestResponse}
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
            <div className="flex flex-col page-transition">
              <div className="flex justify-between items-center mb-1">
                <h1 className="text-4xl font-light">Recipes</h1>
                <div className="flex gap-2">
                  <CreateRecipeButton onClick={() => setShowCreateDialog(true)} />
                  <ImportRecipeButton onClick={() => setShowImportDialog(true)} />
                </div>
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
                <h4 className="text-sm font-medium text-text-standard mb-2">Deeplink</h4>
                <div className="bg-background-muted border border-border-subtle p-3 rounded-lg">
                  <div className="flex items-center justify-between mb-2">
                    <div className="text-sm text-text-muted">
                      Copy this link to share with friends or paste directly in Chrome to open
                    </div>
                    <Button
                      onClick={async () => {
                        try {
                          const deeplink =
                            previewDeeplink || (await generateDeepLink(selectedRecipe.recipe));
                          navigator.clipboard.writeText(deeplink);
                          toastSuccess({
                            title: 'Copied!',
                            msg: 'Recipe deeplink copied to clipboard',
                          });
                        } catch (error) {
                          toastError({
                            title: 'Copy Failed',
                            msg: 'Failed to copy deeplink to clipboard',
                            traceback: error instanceof Error ? error.message : String(error),
                          });
                        }
                      }}
                      variant="ghost"
                      size="sm"
                      className="ml-4 p-2 hover:bg-background-default rounded-lg transition-colors flex items-center"
                    >
                      <span className="text-sm text-text-muted">Copy</span>
                    </Button>
                  </div>
                  <div
                    onClick={async () => {
                      try {
                        const deeplink =
                          previewDeeplink || (await generateDeepLink(selectedRecipe.recipe));
                        navigator.clipboard.writeText(deeplink);
                        toastSuccess({
                          title: 'Copied!',
                          msg: 'Recipe deeplink copied to clipboard',
                        });
                      } catch (error) {
                        toastError({
                          title: 'Copy Failed',
                          msg: 'Failed to copy deeplink to clipboard',
                          traceback: error instanceof Error ? error.message : String(error),
                        });
                      }
                    }}
                    className="text-sm truncate font-mono cursor-pointer text-text-standard"
                  >
                    {previewDeeplink || 'Generating deeplink...'}
                  </div>
                </div>
              </div>

              <div>
                <h4 className="text-sm font-medium text-text-standard mb-2">Description</h4>
                <p className="text-text-muted">{selectedRecipe.recipe.description}</p>
              </div>

              {selectedRecipe.recipe.version && (
                <div>
                  <h4 className="text-sm font-medium text-text-standard mb-2">Version</h4>
                  <div className="bg-background-muted border border-border-subtle p-3 rounded-lg">
                    <span className="text-sm text-text-muted font-mono">
                      {selectedRecipe.recipe.version}
                    </span>
                  </div>
                </div>
              )}

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

              {filteredPreviewParameters && filteredPreviewParameters.length > 0 && (
                <div>
                  <h4 className="text-sm font-medium text-text-standard mb-2">Parameters</h4>
                  <div className="space-y-3">
                    {filteredPreviewParameters.map((param, index) => (
                      <div
                        key={index}
                        className="bg-background-muted border border-border-subtle p-3 rounded-lg"
                      >
                        <div className="flex items-center gap-2 mb-2">
                          <code className="text-sm font-mono bg-background-default px-2 py-1 rounded text-text-standard">
                            {param.key}
                          </code>
                          <span className="text-xs px-2 py-1 rounded bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200">
                            {param.input_type}
                          </span>
                          <span
                            className={`text-xs px-2 py-1 rounded ${
                              param.requirement === 'required'
                                ? 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200'
                                : param.requirement === 'user_prompt'
                                  ? 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200'
                                  : 'bg-gray-100 text-gray-800 dark:bg-gray-900 dark:text-gray-200'
                            }`}
                          >
                            {param.requirement}
                          </span>
                        </div>
                        <p className="text-sm text-text-muted mb-2">{param.description}</p>

                        {param.default && (
                          <div className="text-xs text-text-muted">
                            <span className="font-medium">Default:</span> {param.default}
                          </div>
                        )}

                        {param.input_type === 'select' &&
                          param.options &&
                          param.options.length > 0 && (
                            <div className="text-xs text-text-muted mt-2">
                              <span className="font-medium">Options:</span>
                              <div className="flex flex-wrap gap-1 mt-1">
                                {param.options.map((option, optIndex) => (
                                  <span
                                    key={optIndex}
                                    className="px-2 py-1 bg-background-default border border-border-subtle rounded text-xs"
                                  >
                                    {option}
                                  </span>
                                ))}
                              </div>
                            </div>
                          )}
                      </div>
                    ))}
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

              {selectedRecipe.recipe.extensions && selectedRecipe.recipe.extensions.length > 0 && (
                <div>
                  <h4 className="text-sm font-medium text-text-standard mb-2">Extensions</h4>
                  <div className="space-y-2">
                    {selectedRecipe.recipe.extensions.map((extension, index) => {
                      const extWithDetails = extension as typeof extension & {
                        version?: string;
                        type?: string;
                        bundled?: boolean;
                        cmd?: string;
                        args?: string[];
                        timeout?: number;
                      };

                      return (
                        <div
                          key={index}
                          className="bg-background-muted border border-border-subtle p-3 rounded-lg"
                        >
                          <div className="flex items-center gap-2 mb-1">
                            <span className="font-medium text-text-standard">{extension.name}</span>
                            {extWithDetails.version && (
                              <span className="text-xs px-2 py-1 bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200 rounded">
                                v{extWithDetails.version}
                              </span>
                            )}
                            {extWithDetails.type && (
                              <span className="text-xs px-2 py-1 bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200 rounded">
                                {extWithDetails.type}
                              </span>
                            )}
                            {extWithDetails.bundled && (
                              <span className="text-xs px-2 py-1 bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200 rounded">
                                bundled
                              </span>
                            )}
                          </div>
                          {'description' in extension && extension.description && (
                            <p className="text-sm text-text-muted mb-2">{extension.description}</p>
                          )}

                          {/* Extension command details */}
                          {extWithDetails.cmd && (
                            <div className="text-xs text-text-muted mt-2">
                              <div className="mb-1">
                                <span className="font-medium">Command:</span>{' '}
                                <code className="bg-background-default px-1 rounded">
                                  {extWithDetails.cmd}
                                </code>
                              </div>
                              {extWithDetails.args && extWithDetails.args.length > 0 && (
                                <div className="mb-1">
                                  <span className="font-medium">Args:</span>
                                  <div className="flex flex-wrap gap-1 mt-1">
                                    {extWithDetails.args.map((arg: string, argIndex: number) => (
                                      <code
                                        key={argIndex}
                                        className="px-1 bg-background-default border border-border-subtle rounded text-xs"
                                      >
                                        {arg}
                                      </code>
                                    ))}
                                  </div>
                                </div>
                              )}
                              {extWithDetails.timeout && (
                                <div>
                                  <span className="font-medium">Timeout:</span>{' '}
                                  {extWithDetails.timeout}s
                                </div>
                              )}
                            </div>
                          )}
                        </div>
                      );
                    })}
                  </div>
                </div>
              )}

              {selectedRecipe.recipe.sub_recipes &&
                selectedRecipe.recipe.sub_recipes.length > 0 && (
                  <div>
                    <h4 className="text-sm font-medium text-text-standard mb-2">Sub Recipes</h4>
                    <div className="space-y-3">
                      {selectedRecipe.recipe.sub_recipes.map((subRecipe, index: number) => (
                        <div
                          key={index}
                          className="bg-background-muted border border-border-subtle p-3 rounded-lg"
                        >
                          <div className="flex items-center gap-2 mb-2">
                            <span className="font-medium text-text-standard">{subRecipe.name}</span>
                            <span className="text-xs px-2 py-1 bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200 rounded">
                              sub-recipe
                            </span>
                          </div>

                          <div className="text-sm text-text-muted mb-2">
                            <span className="font-medium">Path:</span>{' '}
                            <code className="bg-background-default px-1 rounded font-mono text-xs">
                              {subRecipe.path}
                            </code>
                          </div>

                          {subRecipe.values && Object.keys(subRecipe.values).length > 0 && (
                            <div className="text-xs text-text-muted mt-2">
                              <span className="font-medium">Parameter Values:</span>
                              <div className="mt-1 space-y-1">
                                {Object.entries(subRecipe.values).map(([key, value]) => (
                                  <div key={key} className="flex items-center gap-2">
                                    <code className="bg-background-default px-1 rounded text-xs">
                                      {key}
                                    </code>
                                    <span>=</span>
                                    <code className="bg-background-default px-1 rounded text-xs">
                                      {String(value)}
                                    </code>
                                  </div>
                                ))}
                              </div>
                            </div>
                          )}

                          {subRecipe.description && (
                            <p className="text-sm text-text-muted mt-2">{subRecipe.description}</p>
                          )}
                        </div>
                      ))}
                    </div>
                  </div>
                )}

              {selectedRecipe.recipe.response && (
                <div>
                  <h4 className="text-sm font-medium text-text-standard mb-2">Response Schema</h4>
                  <div className="bg-background-muted border border-border-subtle p-3 rounded-lg">
                    <pre className="text-sm text-text-muted whitespace-pre-wrap font-mono">
                      {
                        (() => {
                          const response = selectedRecipe.recipe.response;
                          try {
                            if (typeof response === 'string') {
                              return response;
                            }
                            return JSON.stringify(response, null, 2);
                          } catch {
                            return String(response);
                          }
                        })() as string
                      }
                    </pre>
                  </div>
                </div>
              )}
              {selectedRecipe.recipe.context && selectedRecipe.recipe.context.length > 0 && (
                <div>
                  <h4 className="text-sm font-medium text-text-standard mb-2">Context</h4>
                  <div className="space-y-2">
                    {selectedRecipe.recipe.context.map((contextItem, index) => (
                      <div
                        key={index}
                        className="bg-background-muted border border-border-subtle p-2 rounded text-sm text-text-muted font-mono"
                      >
                        {contextItem}
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {selectedRecipe.recipe.author && (
                <div>
                  <h4 className="text-sm font-medium text-text-standard mb-2">Author</h4>
                  <div className="bg-background-muted border border-border-subtle p-3 rounded-lg">
                    {selectedRecipe.recipe.author.contact && (
                      <div className="text-sm text-text-muted mb-1">
                        <span className="font-medium">Contact:</span>{' '}
                        {selectedRecipe.recipe.author.contact}
                      </div>
                    )}
                    {selectedRecipe.recipe.author.metadata && (
                      <div className="text-sm text-text-muted">
                        <span className="font-medium">Metadata:</span>{' '}
                        {selectedRecipe.recipe.author.metadata}
                      </div>
                    )}
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
                  handleLoadRecipe(selectedRecipe.recipe);
                }}
                variant="default"
              >
                Load Recipe
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Use the extracted form components */}
      <ImportRecipeForm
        isOpen={showImportDialog}
        onClose={() => setShowImportDialog(false)}
        onSuccess={loadSavedRecipes}
      />

      <CreateRecipeForm
        isOpen={showCreateDialog}
        onClose={() => setShowCreateDialog(false)}
        onSuccess={loadSavedRecipes}
      />
    </>
  );
}
