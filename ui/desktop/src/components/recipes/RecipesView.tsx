import { useState, useEffect, useMemo } from 'react';
import { listSavedRecipes, convertToLocaleDateString } from '../../recipe/recipe_management';
import {
  FileText,
  Edit,
  Trash2,
  Play,
  Calendar,
  AlertCircle,
  Link,
  Clock,
  Terminal,
  ExternalLink,
  Share2,
  Copy,
  Download,
} from 'lucide-react';
import { ScrollArea } from '../ui/scroll-area';
import { Card } from '../ui/card';
import { Button } from '../ui/button';
import { Skeleton } from '../ui/skeleton';
import { MainPanelLayout } from '../Layout/MainPanelLayout';
import { toastSuccess, toastError } from '../../toasts';
import { useEscapeKey } from '../../hooks/useEscapeKey';
import {
  deleteRecipe,
  RecipeManifest,
  startAgent,
  scheduleRecipe,
  setRecipeSlashCommand,
  recipeToYaml,
} from '../../api';
import ImportRecipeForm, { ImportRecipeButton } from './ImportRecipeForm';
import CreateEditRecipeModal from './CreateEditRecipeModal';
import { generateDeepLink } from '../../recipe';
import { useNavigation } from '../../hooks/useNavigation';
import { CronPicker } from '../schedule/CronPicker';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '../ui/dialog';
import { SearchView } from '../conversation/SearchView';
import cronstrue from 'cronstrue';
import { getInitialWorkingDir } from '../../utils/workingDir';
import {
  trackRecipeDeleted,
  trackRecipeStarted,
  trackRecipeDeeplinkCopied,
  trackRecipeYamlCopied,
  trackRecipeExportedToFile,
  trackRecipeScheduled,
  trackRecipeSlashCommandSet,
  getErrorType,
} from '../../utils/analytics';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  DropdownMenuSeparator,
} from '../ui/dropdown-menu';
import { getSearchShortcutText } from '../../utils/keyboardShortcuts';
import { errorMessage } from '../../utils/conversionUtils';
import { AppEvents } from '../../constants/events';

export default function RecipesView() {
  const setView = useNavigation();
  const [savedRecipes, setSavedRecipes] = useState<RecipeManifest[]>([]);
  const [loading, setLoading] = useState(true);
  const [showSkeleton, setShowSkeleton] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedRecipe, setSelectedRecipe] = useState<RecipeManifest | null>(null);
  const [showEditor, setShowEditor] = useState(false);
  const [showContent, setShowContent] = useState(false);

  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [showImportDialog, setShowImportDialog] = useState(false);

  const [showScheduleDialog, setShowScheduleDialog] = useState(false);
  const [scheduleRecipeManifest, setScheduleRecipeManifest] = useState<RecipeManifest | null>(null);
  const [scheduleCron, setScheduleCron] = useState<string>('');

  const [showSlashCommandDialog, setShowSlashCommandDialog] = useState(false);
  const [slashCommandRecipeManifest, setSlashCommandRecipeManifest] =
    useState<RecipeManifest | null>(null);
  const [slashCommand, setSlashCommand] = useState<string>('');
  const [scheduleValid, setScheduleIsValid] = useState(true);

  const [searchTerm, setSearchTerm] = useState('');

  const filteredRecipes = useMemo(() => {
    if (!searchTerm) return savedRecipes;

    const searchLower = searchTerm.toLowerCase();
    return savedRecipes.filter((recipeManifest) => {
      const { recipe, slash_command } = recipeManifest;
      const title = recipe.title?.toLowerCase() || '';
      const description = recipe.description?.toLowerCase() || '';
      const slashCmd = slash_command?.toLowerCase() || '';

      return (
        title.includes(searchLower) ||
        description.includes(searchLower) ||
        slashCmd.includes(searchLower)
      );
    });
  }, [savedRecipes, searchTerm]);

  useEffect(() => {
    loadSavedRecipes();
  }, []);

  useEscapeKey(showEditor, () => setShowEditor(false));

  useEffect(() => {
    if (!loading && showSkeleton) {
      const timer = setTimeout(() => {
        setShowSkeleton(false);
        setTimeout(() => {
          setShowContent(true);
        }, 50);
      }, 300);

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
      setError(errorMessage(err, 'Failed to load recipes'));
      console.error('Failed to load saved recipes:', err);
    } finally {
      setLoading(false);
    }
  };

  const handleStartRecipeChat = async (recipeId: string) => {
    try {
      const newAgent = await startAgent({
        body: {
          working_dir: getInitialWorkingDir(),
          recipe_id: recipeId,
        },
        throwOnError: true,
      });
      const session = newAgent.data;
      trackRecipeStarted(true, undefined, false);

      window.dispatchEvent(new CustomEvent(AppEvents.SESSION_CREATED, { detail: { session } }));

      setView('pair', {
        disableAnimation: true,
        resumeSessionId: session.id,
      });
    } catch (error) {
      console.error('Failed to load recipe:', error);
      const errorMsg = errorMessage(error, 'Failed to load recipe');
      trackRecipeStarted(false, getErrorType(error), false);
      setError(errorMsg);
    }
  };

  const handleStartRecipeChatInNewWindow = async (recipeId: string) => {
    try {
      window.electron.createChatWindow({
        dir: getInitialWorkingDir(),
        viewType: 'pair',
        recipeId,
      });
      trackRecipeStarted(true, undefined, true);
    } catch (error) {
      console.error('Failed to open recipe in new window:', error);
      trackRecipeStarted(false, getErrorType(error), true);
    }
  };

  const handleDeleteRecipe = async (recipeManifest: RecipeManifest) => {
    const result = await window.electron.showMessageBox({
      type: 'warning',
      buttons: ['Cancel', 'Delete'],
      defaultId: 0,
      title: 'Delete Recipe',
      message: `Are you sure you want to delete "${recipeManifest.recipe.title}"?`,
      detail: 'Recipe file will be deleted.',
    });

    if (result.response !== 1) {
      return;
    }

    try {
      await deleteRecipe({ body: { id: recipeManifest.id } });
      trackRecipeDeleted(true);
      await loadSavedRecipes();
      toastSuccess({
        title: recipeManifest.recipe.title,
        msg: 'Recipe deleted successfully',
      });
    } catch (err) {
      console.error('Failed to delete recipe:', err);
      const errorMsg = errorMessage(err, 'Failed to delete recipe');
      trackRecipeDeleted(false, getErrorType(err));
      setError(errorMsg);
    }
  };

  const handleEditRecipe = async (recipeManifest: RecipeManifest) => {
    setSelectedRecipe(recipeManifest);
    setShowEditor(true);
  };

  const handleEditorClose = (wasSaved?: boolean) => {
    setShowEditor(false);
    setSelectedRecipe(null);
    if (wasSaved) {
      loadSavedRecipes();
    }
  };

  const handleCopyDeeplink = async (recipeManifest: RecipeManifest) => {
    try {
      const deeplink = await generateDeepLink(recipeManifest.recipe);
      await navigator.clipboard.writeText(deeplink);
      trackRecipeDeeplinkCopied(true);
      toastSuccess({
        title: 'Deeplink copied',
        msg: 'Recipe deeplink has been copied to clipboard',
      });
    } catch (error) {
      console.error('Failed to copy deeplink:', error);
      trackRecipeDeeplinkCopied(false, getErrorType(error));
      toastError({
        title: 'Copy failed',
        msg: 'Failed to copy deeplink to clipboard',
      });
    }
  };

  const handleCopyYaml = async (recipeManifest: RecipeManifest) => {
    try {
      const response = await recipeToYaml({
        body: { recipe: recipeManifest.recipe },
        throwOnError: true,
      });

      if (!response.data?.yaml) {
        throw new Error('No YAML data returned from API');
      }

      await navigator.clipboard.writeText(response.data.yaml);
      trackRecipeYamlCopied(true);
      toastSuccess({
        title: 'YAML copied',
        msg: 'Recipe YAML has been copied to clipboard',
      });
    } catch (error) {
      console.error('Failed to copy YAML:', error);
      trackRecipeYamlCopied(false, getErrorType(error));
      toastError({
        title: 'Copy failed',
        msg: 'Failed to copy recipe YAML to clipboard',
      });
    }
  };

  const handleExportFile = async (recipeManifest: RecipeManifest) => {
    try {
      const response = await recipeToYaml({
        body: { recipe: recipeManifest.recipe },
        throwOnError: true,
      });

      if (!response.data?.yaml) {
        throw new Error('No YAML data returned from API');
      }

      const sanitizedTitle = (recipeManifest.recipe.title || 'recipe')
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, '-')
        .replace(/^-|-$/g, '');

      const filename = `${sanitizedTitle}.yaml`;

      const result = await window.electron.showSaveDialog({
        title: 'Export Recipe',
        defaultPath: filename,
        filters: [
          { name: 'YAML Files', extensions: ['yaml', 'yml'] },
          { name: 'All Files', extensions: ['*'] },
        ],
      });

      if (!result.canceled && result.filePath) {
        await window.electron.writeFile(result.filePath, response.data.yaml);
        trackRecipeExportedToFile(true);
        toastSuccess({
          title: 'Recipe exported',
          msg: `Recipe saved to ${result.filePath}`,
        });
      }
    } catch (error) {
      console.error('Failed to export recipe:', error);
      trackRecipeExportedToFile(false, getErrorType(error));
      toastError({
        title: 'Export failed',
        msg: 'Failed to export recipe to file',
      });
    }
  };

  const handleOpenScheduleDialog = (recipeManifest: RecipeManifest) => {
    setScheduleRecipeManifest(recipeManifest);
    setScheduleCron(recipeManifest.schedule_cron || '0 0 14 * * *');
    setShowScheduleDialog(true);
  };

  const handleSaveSchedule = async () => {
    if (!scheduleRecipeManifest) return;

    const action = scheduleRecipeManifest.schedule_cron ? 'edit' : 'add';

    try {
      await scheduleRecipe({
        body: {
          id: scheduleRecipeManifest.id,
          cron_schedule: scheduleCron,
        },
      });

      trackRecipeScheduled(true, action);
      toastSuccess({
        title: 'Schedule saved',
        msg: `Recipe will run ${getReadableCron(scheduleCron)}`,
      });

      setShowScheduleDialog(false);
      setScheduleRecipeManifest(null);
      await loadSavedRecipes();
    } catch (error) {
      console.error('Failed to save schedule:', error);
      const errorMsg = errorMessage(error, 'Failed to save schedule');
      trackRecipeScheduled(false, action, getErrorType(error));
      setError(errorMsg);
    }
  };

  const handleRemoveSchedule = async () => {
    if (!scheduleRecipeManifest) return;

    try {
      await scheduleRecipe({
        body: {
          id: scheduleRecipeManifest.id,
          cron_schedule: null,
        },
      });

      trackRecipeScheduled(true, 'remove');
      toastSuccess({
        title: 'Schedule removed',
        msg: 'Recipe will no longer run automatically',
      });

      setShowScheduleDialog(false);
      setScheduleRecipeManifest(null);
      await loadSavedRecipes();
    } catch (error) {
      console.error('Failed to remove schedule:', error);
      const errorMsg = errorMessage(error, 'Failed to remove schedule');
      trackRecipeScheduled(false, 'remove', getErrorType(error));
      setError(errorMsg);
    }
  };

  const handleOpenSlashCommandDialog = (recipeManifest: RecipeManifest) => {
    setSlashCommandRecipeManifest(recipeManifest);
    setSlashCommand(recipeManifest.slash_command || '');
    setShowSlashCommandDialog(true);
  };

  const handleSaveSlashCommand = async () => {
    if (!slashCommandRecipeManifest) return;

    const action = slashCommand
      ? slashCommandRecipeManifest.slash_command
        ? 'edit'
        : 'add'
      : 'remove';

    try {
      await setRecipeSlashCommand({
        body: {
          id: slashCommandRecipeManifest.id,
          slash_command: slashCommand || null,
        },
      });

      trackRecipeSlashCommandSet(true, action);
      toastSuccess({
        title: 'Slash command saved',
        msg: slashCommand ? `Use /${slashCommand} to run this recipe` : 'Slash command removed',
      });

      setShowSlashCommandDialog(false);
      setSlashCommandRecipeManifest(null);
      await loadSavedRecipes();
    } catch (error) {
      console.error('Failed to save slash command:', error);
      const errorMsg = errorMessage(error, 'Failed to save slash command');
      trackRecipeSlashCommandSet(false, action, getErrorType(error));
      setError(errorMsg);
    }
  };

  const handleRemoveSlashCommand = async () => {
    if (!slashCommandRecipeManifest) return;

    try {
      await setRecipeSlashCommand({
        body: {
          id: slashCommandRecipeManifest.id,
          slash_command: null,
        },
      });

      trackRecipeSlashCommandSet(true, 'remove');
      toastSuccess({
        title: 'Slash command removed',
        msg: 'Recipe slash command has been removed',
      });

      setShowSlashCommandDialog(false);
      setSlashCommandRecipeManifest(null);
      await loadSavedRecipes();
    } catch (error) {
      console.error('Failed to remove slash command:', error);
      const errorMsg = errorMessage(error, 'Failed to remove slash command');
      trackRecipeSlashCommandSet(false, 'remove', getErrorType(error));
      setError(errorMsg);
    }
  };

  const getReadableCron = (cron: string): string => {
    try {
      const cronWithoutSeconds = cron.split(' ').slice(1).join(' ');
      return cronstrue.toString(cronWithoutSeconds).toLowerCase();
    } catch {
      return cron;
    }
  };

  const RecipeItem = ({
    recipeManifestResponse,
    recipeManifestResponse: { recipe, last_modified: lastModified, schedule_cron, slash_command },
  }: {
    recipeManifestResponse: RecipeManifest;
  }) => (
    <Card className="py-2 px-4 mb-2 bg-background-primary border-none hover:bg-background-secondary transition-all duration-150">
      <div className="flex justify-between items-start gap-4">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2 mb-1">
            <h3 className="text-base truncate max-w-[50vw]">{recipe.title}</h3>
          </div>
          <p className="text-text-secondary text-sm mb-2 line-clamp-2">{recipe.description}</p>
          <div className="flex flex-col gap-1 text-xs text-text-secondary">
            <div className="flex items-center">
              <Calendar className="w-3 h-3 mr-1" />
              {convertToLocaleDateString(lastModified)}
            </div>
            {(schedule_cron || slash_command) && (
              <div className="flex items-center gap-3">
                {schedule_cron && (
                  <div className="flex items-center text-blue-600 dark:text-blue-400">
                    <Clock className="w-3 h-3 mr-1" />
                    Runs {getReadableCron(schedule_cron)}
                  </div>
                )}
                {slash_command && (
                  <div className="flex items-center text-purple-600 dark:text-purple-400">
                    /{slash_command}
                  </div>
                )}
              </div>
            )}
          </div>
        </div>

        <Button
          onClick={(e) => {
            e.stopPropagation();
            handleOpenSlashCommandDialog(recipeManifestResponse);
          }}
          variant={slash_command ? 'default' : 'outline'}
          size="sm"
          className="h-8 w-8 p-0"
          title={slash_command ? 'Edit slash command' : 'Add slash command'}
        >
          <Terminal className="w-4 h-4" />
        </Button>

        <div className="flex items-center gap-2 shrink-0">
          <Button
            onClick={async (e) => {
              e.stopPropagation();
              await handleStartRecipeChat(recipeManifestResponse.id);
            }}
            size="sm"
            className="h-8 w-8 p-0"
            title="Use recipe"
          >
            <Play className="w-4 h-4" />
          </Button>
          <Button
            onClick={async (e) => {
              e.stopPropagation();
              await handleStartRecipeChatInNewWindow(recipeManifestResponse.id);
            }}
            variant="outline"
            size="sm"
            className="h-8 w-8 p-0"
            title="Open in new window"
          >
            <ExternalLink className="w-4 h-4" />
          </Button>
          <Button
            onClick={async (e) => {
              e.stopPropagation();
              await handleEditRecipe(recipeManifestResponse);
            }}
            variant="outline"
            size="sm"
            className="h-8 w-8 p-0"
            title="Edit recipe"
          >
            <Edit className="w-4 h-4" />
          </Button>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                onClick={(e) => e.stopPropagation()}
                variant="outline"
                size="sm"
                className="h-8 w-8 p-0"
                title="Share recipe"
              >
                <Share2 className="w-4 h-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" onClick={(e) => e.stopPropagation()}>
              <DropdownMenuItem onClick={() => handleCopyDeeplink(recipeManifestResponse)}>
                <Link className="w-4 h-4" />
                Copy Deeplink
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => handleCopyYaml(recipeManifestResponse)}>
                <Copy className="w-4 h-4" />
                Copy YAML
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem onClick={() => handleExportFile(recipeManifestResponse)}>
                <Download className="w-4 h-4" />
                Export to File
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
          <Button
            onClick={(e) => {
              e.stopPropagation();
              handleOpenScheduleDialog(recipeManifestResponse);
            }}
            variant={schedule_cron ? 'default' : 'outline'}
            size="sm"
            className="h-8 w-8 p-0"
            title={schedule_cron ? 'Edit schedule' : 'Add schedule'}
          >
            <Clock className="w-4 h-4" />
          </Button>
          <Button
            onClick={(e) => {
              e.stopPropagation();
              handleDeleteRecipe(recipeManifestResponse);
            }}
            variant="ghost"
            size="sm"
            className="h-8 w-8 p-0 text-red-500 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20"
            title="Delete recipe"
          >
            <Trash2 className="w-4 h-4" />
          </Button>
        </div>
      </div>
    </Card>
  );

  const RecipeSkeleton = () => (
    <Card className="p-2 mb-2 bg-background-primary">
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
        <div className="flex flex-col items-center justify-center h-full text-text-secondary">
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
          <p className="text-sm text-text-secondary">Recipe saved from chats will show up here.</p>
        </div>
      );
    }

    if (filteredRecipes.length === 0 && searchTerm) {
      return (
        <div className="flex flex-col items-center justify-center h-full text-text-secondary mt-4">
          <FileText className="h-12 w-12 mb-4" />
          <p className="text-lg mb-2">No matching recipes found</p>
          <p className="text-sm">Try adjusting your search terms</p>
        </div>
      );
    }

    return (
      <div className="space-y-2">
        {filteredRecipes.map((recipeManifestResponse: RecipeManifest) => (
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
          <div className="bg-background-primary px-8 pb-8 pt-16">
            <div className="flex flex-col page-transition">
              <div className="flex justify-between items-center mb-1">
                <h1 className="text-4xl font-light">Recipes</h1>
                <div className="flex gap-2">
                  <Button
                    onClick={() => setShowCreateDialog(true)}
                    variant="outline"
                    size="sm"
                    className="flex items-center gap-2"
                  >
                    <FileText className="w-4 h-4" />
                    Create Recipe
                  </Button>
                  <ImportRecipeButton onClick={() => setShowImportDialog(true)} />
                </div>
              </div>
              <p className="text-sm text-text-secondary mb-1">
                View and manage your saved recipes to quickly start new sessions with predefined
                configurations. {getSearchShortcutText()} to search.
              </p>
            </div>
          </div>

          <div className="flex-1 min-h-0 relative px-8">
            <ScrollArea className="h-full">
              <SearchView onSearch={(term) => setSearchTerm(term)} placeholder="Search recipes...">
                <div
                  className={`h-full relative transition-all duration-300 ${
                    showContent ? 'opacity-100 animate-in fade-in ' : 'opacity-0'
                  }`}
                >
                  {renderContent()}
                </div>
              </SearchView>
            </ScrollArea>
          </div>
        </div>
      </MainPanelLayout>

      {showEditor && selectedRecipe && (
        <CreateEditRecipeModal
          isOpen={showEditor}
          onClose={handleEditorClose}
          recipe={selectedRecipe.recipe}
          recipeId={selectedRecipe.id}
        />
      )}

      <ImportRecipeForm
        isOpen={showImportDialog}
        onClose={() => setShowImportDialog(false)}
        onSuccess={loadSavedRecipes}
      />

      {showCreateDialog && (
        <CreateEditRecipeModal
          isOpen={showCreateDialog}
          onClose={() => {
            setShowCreateDialog(false);
            loadSavedRecipes();
          }}
          isCreateMode={true}
        />
      )}

      {showScheduleDialog && scheduleRecipeManifest && (
        <Dialog open={showScheduleDialog} onOpenChange={setShowScheduleDialog}>
          <DialogContent className="max-w-md">
            <DialogHeader>
              <DialogTitle>
                {scheduleRecipeManifest.schedule_cron ? 'Edit' : 'Add'} Schedule
              </DialogTitle>
            </DialogHeader>
            <div className="space-y-4">
              <CronPicker
                schedule={
                  scheduleRecipeManifest.schedule_cron
                    ? {
                        id: scheduleRecipeManifest.id,
                        source: '',
                        cron: scheduleRecipeManifest.schedule_cron,
                        last_run: null,
                        currently_running: false,
                        paused: false,
                      }
                    : null
                }
                onChange={setScheduleCron}
                isValid={setScheduleIsValid}
              />
              <div className="flex gap-2 justify-end">
                {scheduleRecipeManifest.schedule_cron && (
                  <Button variant="outline" onClick={handleRemoveSchedule}>
                    Remove Schedule
                  </Button>
                )}
                <Button variant="outline" onClick={() => setShowScheduleDialog(false)}>
                  Cancel
                </Button>
                <Button onClick={handleSaveSchedule} disabled={!scheduleValid}>
                  Save
                </Button>
              </div>
            </div>
          </DialogContent>
        </Dialog>
      )}

      {showSlashCommandDialog && slashCommandRecipeManifest && (
        <Dialog open={showSlashCommandDialog} onOpenChange={setShowSlashCommandDialog}>
          <DialogContent className="max-w-md">
            <DialogHeader>
              <DialogTitle>Slash Command</DialogTitle>
            </DialogHeader>
            <div className="space-y-4">
              <div>
                <p className="text-sm text-muted-foreground mb-3">
                  Set a slash command to quickly run this recipe from any chat
                </p>
                <div className="flex gap-2 items-center">
                  <span className="text-muted-foreground">/</span>
                  <input
                    type="text"
                    value={slashCommand}
                    onChange={(e) => setSlashCommand(e.target.value)}
                    placeholder="command-name"
                    className="flex-1 px-3 py-2 border rounded text-sm"
                  />
                </div>
                {slashCommand && (
                  <p className="text-xs text-muted-foreground mt-2">
                    Use /{slashCommand} in any chat to run this recipe
                  </p>
                )}
              </div>

              <div className="flex gap-2 justify-end">
                {slashCommandRecipeManifest.slash_command && (
                  <Button variant="outline" onClick={handleRemoveSlashCommand}>
                    Remove
                  </Button>
                )}
                <Button variant="outline" onClick={() => setShowSlashCommandDialog(false)}>
                  Cancel
                </Button>
                <Button onClick={handleSaveSlashCommand}>Save</Button>
              </div>
            </div>
          </DialogContent>
        </Dialog>
      )}
    </>
  );
}
