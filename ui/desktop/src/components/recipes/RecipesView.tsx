import { useState, useEffect } from 'react';
import { listSavedRecipes, convertToLocaleDateString } from '../../recipe/recipe_management';
import { FileText, Edit, Trash2, Play, Calendar, AlertCircle, Link, Pause, Play as PlayIcon, Settings } from 'lucide-react';
import { ScrollArea } from '../ui/scroll-area';
import { Card } from '../ui/card';
import { Button } from '../ui/button';
import { Skeleton } from '../ui/skeleton';
import { MainPanelLayout } from '../Layout/MainPanelLayout';
import { toastSuccess, toastError } from '../../toasts';
import { useEscapeKey } from '../../hooks/useEscapeKey';
import { deleteRecipe, RecipeManifestResponse } from '../../api';
import ImportRecipeForm, { ImportRecipeButton } from './ImportRecipeForm';
import CreateEditRecipeModal from './CreateEditRecipeModal';
import { generateDeepLink, Recipe } from '../../recipe';
import { ScheduleFromRecipeModal } from '../schedule/ScheduleFromRecipeModal';
import { useNavigation } from '../../hooks/useNavigation';
import { listSchedules, ScheduledJob, pauseSchedule, unpauseSchedule } from '../../schedule';
import { matchRecipesWithSchedules } from '../../utils/recipeScheduleUtils';
import { ScheduleBadge } from './ScheduleBadge';

export default function RecipesView() {
  const setView = useNavigation();
  const [savedRecipes, setSavedRecipes] = useState<RecipeManifestResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const [showSkeleton, setShowSkeleton] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedRecipe, setSelectedRecipe] = useState<RecipeManifestResponse | null>(null);
  const [showEditor, setShowEditor] = useState(false);
  const [showContent, setShowContent] = useState(false);

  // Schedule-related state
  const [schedules, setSchedules] = useState<ScheduledJob[]>([]);
  const [recipeScheduleMap, setRecipeScheduleMap] = useState<Map<string, ScheduledJob>>(new Map());
  const [loadingSchedules, setLoadingSchedules] = useState(false);

  // Form dialog states
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [showImportDialog, setShowImportDialog] = useState(false);
  const [showScheduleModal, setShowScheduleModal] = useState(false);
  const [selectedRecipeForSchedule, setSelectedRecipeForSchedule] = useState<Recipe | null>(null);

  useEffect(() => {
    loadSavedRecipes();
  }, []);

  // Handle Esc key for editor modal
  useEscapeKey(showEditor, () => setShowEditor(false));

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

  const loadSchedules = async () => {
    try {
      setLoadingSchedules(true);
      const fetchedSchedules = await listSchedules();
      setSchedules(fetchedSchedules);
      return fetchedSchedules;
    } catch (err) {
      console.error('Failed to load schedules:', err);
      // Don't show error toast for schedules - it's not critical
      return [];
    } finally {
      setLoadingSchedules(false);
    }
  };

  const loadSavedRecipes = async () => {
    try {
      setLoading(true);
      setShowSkeleton(true);
      setShowContent(false);
      setError(null);
      
      // Load recipes and schedules in parallel
      const [recipeManifestResponses, fetchedSchedules] = await Promise.all([
        listSavedRecipes(),
        loadSchedules()
      ]);
      
      setSavedRecipes(recipeManifestResponses);
      
      // Match recipes with schedules
      const scheduleMap = matchRecipesWithSchedules(recipeManifestResponses, fetchedSchedules);
      setRecipeScheduleMap(scheduleMap);
      
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load recipes');
      console.error('Failed to load saved recipes:', err);
    } finally {
      setLoading(false);
    }
  };

  const handleLoadRecipe = async (recipe: Recipe, recipeId: string) => {
    try {
      window.electron.createChatWindow(
        undefined, // query
        undefined, // dir
        undefined, // version
        undefined, // resumeSessionId
        recipe, // recipe config
        undefined, // view type,
        recipeId // recipe id
      );
    } catch (err) {
      console.error('Failed to load recipe:', err);
      setError(err instanceof Error ? err.message : 'Failed to load recipe');
    }
  };

  const handleDeleteRecipe = async (recipeManifest: RecipeManifestResponse) => {
    const confirmed = window.confirm(
      `Are you sure you want to delete "${recipeManifest.recipe.title}"?`
    );
    if (!confirmed) return;

    try {
      await deleteRecipe(recipeManifest.id);
      toastSuccess({
        title: 'Command deleted',
        msg: `"${recipeManifest.recipe.title}" has been deleted`,
      });
      await loadSavedRecipes();
    } catch (err) {
      console.error('Failed to delete recipe:', err);
      setError(err instanceof Error ? err.message : 'Failed to delete recipe');
    }
  };

  const handleEditRecipe = (recipeManifest: RecipeManifestResponse) => {
    setSelectedRecipe(recipeManifest);
    setShowEditor(true);
  };

  const handleCloseEditor = () => {
    setShowEditor(false);
    setSelectedRecipe(null);
    loadSavedRecipes();
  };

  const handleCopyDeeplink = async (recipeManifest: RecipeManifestResponse) => {
    try {
      const deeplink = await generateDeepLink(recipeManifest.recipe);
      await navigator.clipboard.writeText(deeplink);
      toastSuccess({
        title: 'Deeplink copied',
        msg: 'Recipe deeplink copied to clipboard',
      });
    } catch (error) {
      console.error('Failed to copy deeplink:', error);
      toastSuccess({
        title: 'Copy failed',
        msg: 'Failed to copy deeplink to clipboard',
      });
    }
  };

  const handleScheduleRecipe = (recipe: Recipe) => {
    setSelectedRecipeForSchedule(recipe);
    setShowScheduleModal(true);
  };

  const handleCreateScheduleFromRecipe = async (deepLink: string) => {
    // Navigate to schedules view with the deep link in state
    setView('commands', { tab: 'scheduler', pendingScheduleDeepLink: deepLink });

    setShowScheduleModal(false);
    setSelectedRecipeForSchedule(null);
  };

  const handlePauseSchedule = async (scheduleId: string, recipeName: string) => {
    try {
      await pauseSchedule(scheduleId);
      toastSuccess({
        title: 'Schedule paused',
        msg: `Schedule for "${recipeName}" has been paused`,
      });
      await loadSavedRecipes(); // Reload to update schedule status
    } catch (err) {
      console.error('Failed to pause schedule:', err);
      toastError({
        title: 'Failed to pause schedule',
        msg: err instanceof Error ? err.message : 'Unknown error',
      });
    }
  };

  const handleResumeSchedule = async (scheduleId: string, recipeName: string) => {
    try {
      await unpauseSchedule(scheduleId);
      toastSuccess({
        title: 'Schedule resumed',
        msg: `Schedule for "${recipeName}" has been resumed`,
      });
      await loadSavedRecipes(); // Reload to update schedule status
    } catch (err) {
      console.error('Failed to resume schedule:', err);
      toastError({
        title: 'Failed to resume schedule',
        msg: err instanceof Error ? err.message : 'Unknown error',
      });
    }
  };

  const handleEditSchedule = (scheduleId: string) => {
    // Navigate to scheduler tab with the schedule selected
    setView('commands', { tab: 'scheduler', selectedScheduleId: scheduleId });
  };

  // Render a recipe item
  const RecipeItem = ({
    recipeManifestResponse,
    recipeManifestResponse: { recipe, lastModified },
  }: {
    recipeManifestResponse: RecipeManifestResponse;
  }) => {
    const schedule = recipeScheduleMap.get(recipeManifestResponse.id);
    const hasSchedule = !!schedule;

    return (
      <Card className="py-2 px-4 mb-2 bg-background-default border-none hover:bg-background-muted transition-all duration-150">
        <div className="flex justify-between items-start gap-4">
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2 mb-1">
              <h3 className="text-base truncate max-w-[50vw]">{recipe.title}</h3>
              {hasSchedule && <ScheduleBadge schedule={schedule} showNextRun={false} />}
            </div>
            <p className="text-text-muted text-sm mb-2 line-clamp-2">{recipe.description}</p>
            <div className="flex items-center gap-4 text-xs text-text-muted">
              <div className="flex items-center">
                <Calendar className="w-3 h-3 mr-1" />
                {convertToLocaleDateString(lastModified)}
              </div>
              {hasSchedule && schedule && (
                <ScheduleBadge schedule={schedule} showNextRun={true} className="text-xs" />
              )}
            </div>
          </div>

          <div className="flex items-center gap-2 shrink-0">
            <Button
              onClick={(e) => {
                e.stopPropagation();
                handleLoadRecipe(recipe, recipeManifestResponse.id);
              }}
              size="sm"
              className="h-8 w-8 p-0"
              title="Run command"
            >
              <Play className="w-4 h-4" />
            </Button>
            <Button
              onClick={(e) => {
                e.stopPropagation();
                handleEditRecipe(recipeManifestResponse);
              }}
              variant="outline"
              size="sm"
              className="h-8 w-8 p-0"
              title="Edit command"
            >
              <Edit className="w-4 h-4" />
            </Button>
            <Button
              onClick={(e) => {
                e.stopPropagation();
                handleCopyDeeplink(recipeManifestResponse);
              }}
              variant="outline"
              size="sm"
              className="h-8 w-8 p-0"
              title="Copy deeplink"
            >
              <Link className="w-4 h-4" />
            </Button>
            
            {/* Schedule-related buttons */}
            {hasSchedule && schedule ? (
              <>
                <Button
                  onClick={(e) => {
                    e.stopPropagation();
                    if (schedule.paused) {
                      handleResumeSchedule(schedule.id, recipe.title);
                    } else {
                      handlePauseSchedule(schedule.id, recipe.title);
                    }
                  }}
                  variant="outline"
                  size="sm"
                  className="h-8 w-8 p-0"
                  title={schedule.paused ? 'Resume schedule' : 'Pause schedule'}
                >
                  {schedule.paused ? (
                    <PlayIcon className="w-4 h-4" />
                  ) : (
                    <Pause className="w-4 h-4" />
                  )}
                </Button>
                <Button
                  onClick={(e) => {
                    e.stopPropagation();
                    handleEditSchedule(schedule.id);
                  }}
                  variant="outline"
                  size="sm"
                  className="h-8 w-8 p-0"
                  title="Edit schedule"
                >
                  <Settings className="w-4 h-4" />
                </Button>
              </>
            ) : (
              <Button
                onClick={(e) => {
                  e.stopPropagation();
                  handleScheduleRecipe(recipe);
                }}
                variant="outline"
                size="sm"
                className="h-8 w-8 p-0"
                title="Create schedule"
              >
                <Calendar className="w-4 h-4" />
              </Button>
            )}
            
            <Button
              onClick={(e) => {
                e.stopPropagation();
                handleDeleteRecipe(recipeManifestResponse);
              }}
              variant="ghost"
              size="sm"
              className="h-8 w-8 p-0 text-red-500 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20"
              title="Delete command"
            >
              <Trash2 className="w-4 h-4" />
            </Button>
          </div>
        </div>
      </Card>
    );
  };

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
          <Skeleton className="h-8 w-8" />
          <Skeleton className="h-8 w-8" />
          <Skeleton className="h-8 w-8" />
          <Skeleton className="h-8 w-8" />
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
          <p className="text-lg mb-2">Error Loading Commands</p>
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
          <p className="text-lg">No saved commands</p>
          <p className="text-sm text-text-muted">Commands saved from chats will show up here.</p>
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
                <h1 className="text-4xl font-light">Commands</h1>
                <div className="flex gap-2">
                  <Button
                    onClick={() => setShowCreateDialog(true)}
                    variant="outline"
                    size="sm"
                    className="flex items-center gap-2"
                  >
                    <FileText className="w-4 h-4" />
                    Create Command
                  </Button>
                  <ImportRecipeButton onClick={() => setShowImportDialog(true)} />
                </div>
              </div>
              <p className="text-sm text-text-muted mb-1">
                View and manage your commands and schedules to quickly start new sessions with predefined
                configurations.
              </p>
            </div>
          </div>

          <ScrollArea className="flex-1 px-8">
            <div
              className={`transition-opacity duration-300 ${
                showContent ? 'opacity-100' : 'opacity-0'
              }`}
            >
              {renderContent()}
            </div>
          </ScrollArea>
        </div>
      </MainPanelLayout>

      {/* Modals */}
      {showCreateDialog && (
        <CreateEditRecipeModal
          isOpen={showCreateDialog}
          onClose={() => {
            setShowCreateDialog(false);
            loadSavedRecipes();
          }}
        />
      )}

      {showImportDialog && (
        <ImportRecipeForm
          isOpen={showImportDialog}
          onClose={() => {
            setShowImportDialog(false);
            loadSavedRecipes();
          }}
        />
      )}

      {showEditor && selectedRecipe && (
        <CreateEditRecipeModal
          isOpen={showEditor}
          onClose={handleCloseEditor}
          recipe={selectedRecipe.recipe}
          recipeId={selectedRecipe.id}
        />
      )}

      {showScheduleModal && selectedRecipeForSchedule && (
        <ScheduleFromRecipeModal
          isOpen={showScheduleModal}
          onClose={() => {
            setShowScheduleModal(false);
            setSelectedRecipeForSchedule(null);
          }}
          recipe={selectedRecipeForSchedule}
          onCreateSchedule={handleCreateScheduleFromRecipe}
        />
      )}
    </>
  );
}
