import React, { useState, useEffect, FormEvent, useCallback } from 'react';
import { Card } from '../ui/card';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { ScheduledJob } from '../../schedule';
import { CronPicker } from './CronPicker';
import { Recipe, decodeRecipe } from '../../recipe';
import { getStorageDirectory } from '../../recipe/recipe_management';
import ClockIcon from '../../assets/clock-icon.svg';
import * as yaml from 'yaml';

export interface NewSchedulePayload {
  id: string;
  recipe_source: string;
  cron: string;
  execution_mode?: string;
}

interface ScheduleModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (payload: NewSchedulePayload | string) => Promise<void>;
  schedule: ScheduledJob | null;
  isLoadingExternally: boolean;
  apiErrorExternally: string | null;
  initialDeepLink: string | null;
}

type SourceType = 'file' | 'deeplink';

interface CleanExtension {
  name: string;
  type: 'stdio' | 'sse' | 'builtin' | 'frontend' | 'streamable_http';
  cmd?: string;
  args?: string[];
  uri?: string;
  display_name?: string;
  tools?: unknown[];
  instructions?: string;
  env_keys?: string[];
  timeout?: number;
  description?: string;
  bundled?: boolean;
}

interface CleanRecipe {
  title: string;
  description: string;
  instructions?: string;
  prompt?: string;
  activities?: string[];
  extensions?: CleanExtension[];
  author?: {
    contact?: string;
    metadata?: string;
  };
  schedule?: {
    window_title?: string;
    working_directory?: string;
  };
}

async function parseDeepLink(deepLink: string): Promise<Recipe | null> {
  try {
    const url = new URL(deepLink);
    if (url.protocol !== 'goose:' || (url.hostname !== 'bot' && url.hostname !== 'recipe')) {
      return null;
    }

    const recipeParam = url.searchParams.get('config');
    if (!recipeParam) {
      return null;
    }

    return await decodeRecipe(recipeParam);
  } catch (error) {
    console.error('Failed to parse deep link:', error);
    return null;
  }
}

function recipeToYaml(recipe: Recipe): string {
  const cleanRecipe: CleanRecipe = {
    title: recipe.title,
    description: recipe.description,
  };

  if (recipe.instructions) {
    cleanRecipe.instructions = recipe.instructions;
  }

  if (recipe.prompt) {
    cleanRecipe.prompt = recipe.prompt;
  }

  if (recipe.activities && recipe.activities.length > 0) {
    cleanRecipe.activities = recipe.activities;
  }

  if (recipe.extensions && recipe.extensions.length > 0) {
    cleanRecipe.extensions = recipe.extensions.map((ext) => {
      const cleanExt: CleanExtension = {
        name: ext.name,
        type: 'builtin',
      };

      if ('type' in ext && ext.type) {
        cleanExt.type = ext.type as CleanExtension['type'];

        const extAny = ext as Record<string, unknown>;

        if (ext.type === 'sse' && extAny.uri) {
          cleanExt.uri = extAny.uri as string;
        } else if (ext.type === 'streamable_http' && extAny.uri) {
          cleanExt.uri = extAny.uri as string;
        } else if (ext.type === 'stdio') {
          if (extAny.cmd) {
            cleanExt.cmd = extAny.cmd as string;
          }
          if (extAny.args) {
            cleanExt.args = extAny.args as string[];
          }
        } else if (ext.type === 'builtin' && extAny.display_name) {
          cleanExt.display_name = extAny.display_name as string;
        }

        if ((ext.type as string) === 'frontend') {
          if (extAny.tools) {
            cleanExt.tools = extAny.tools as unknown[];
          }
          if (extAny.instructions) {
            cleanExt.instructions = extAny.instructions as string;
          }
        }
      } else {
        const extAny = ext as Record<string, unknown>;

        if (extAny.cmd) {
          cleanExt.type = 'stdio';
          cleanExt.cmd = extAny.cmd as string;
          if (extAny.args) {
            cleanExt.args = extAny.args as string[];
          }
        } else if (extAny.command) {
          cleanExt.type = 'stdio';
          cleanExt.cmd = extAny.command as string;
        } else if (extAny.uri) {
          cleanExt.type = 'streamable_http';
          cleanExt.uri = extAny.uri as string;
        } else if (extAny.tools) {
          cleanExt.type = 'frontend';
          cleanExt.tools = extAny.tools as unknown[];
          if (extAny.instructions) {
            cleanExt.instructions = extAny.instructions as string;
          }
        } else {
          cleanExt.type = 'builtin';
        }
      }

      if ('env_keys' in ext && ext.env_keys && ext.env_keys.length > 0) {
        cleanExt.env_keys = ext.env_keys;
      }

      if ('timeout' in ext && ext.timeout) {
        cleanExt.timeout = ext.timeout as number;
      }

      if ('description' in ext && ext.description) {
        cleanExt.description = ext.description as string;
      }

      if ('bundled' in ext && ext.bundled !== undefined) {
        cleanExt.bundled = ext.bundled as boolean;
      }

      return cleanExt;
    });
  }

  if (recipe.author) {
    cleanRecipe.author = {
      contact: recipe.author.contact || undefined,
      metadata: recipe.author.metadata || undefined,
    };
  }

  cleanRecipe.schedule = {
    window_title: `${recipe.title} - Scheduled`,
  };

  return yaml.stringify(cleanRecipe);
}

const modalLabelClassName = 'block text-sm font-medium text-text-prominent mb-1';

export const ScheduleModal: React.FC<ScheduleModalProps> = ({
  isOpen,
  onClose,
  onSubmit,
  schedule,
  isLoadingExternally,
  apiErrorExternally,
  initialDeepLink,
}) => {
  const isEditMode = !!schedule;

  const [scheduleId, setScheduleId] = useState<string>('');
  const [sourceType, setSourceType] = useState<SourceType>('file');
  const [recipeSourcePath, setRecipeSourcePath] = useState<string>('');
  const [deepLinkInput, setDeepLinkInput] = useState<string>('');
  const [parsedRecipe, setParsedRecipe] = useState<Recipe | null>(null);
  const [cronExpression, setCronExpression] = useState<string>('0 0 14 * * *');
  const [internalValidationError, setInternalValidationError] = useState<string | null>(null);
  const [isValid, setIsValid] = useState(true);

  const handleDeepLinkChange = useCallback(async (value: string) => {
    setDeepLinkInput(value);
    setInternalValidationError(null);

    if (value.trim()) {
      try {
        const recipe = await parseDeepLink(value.trim());
        if (recipe) {
          setParsedRecipe(recipe);
          if (recipe.title) {
            const cleanId = recipe.title
              .toLowerCase()
              .replace(/[^a-z0-9-]/g, '-')
              .replace(/-+/g, '-');
            setScheduleId(cleanId);
          }
        } else {
          setParsedRecipe(null);
          setInternalValidationError(
            'Invalid deep link format. Please use a goose://bot or goose://recipe link.'
          );
        }
      } catch {
        setParsedRecipe(null);
        setInternalValidationError(
          'Failed to parse deep link. Please ensure using a goose://bot or goose://recipe link and try again.'
        );
      }
    } else {
      setParsedRecipe(null);
    }
  }, []);

  useEffect(() => {
    if (isOpen) {
      if (schedule) {
        setScheduleId(schedule.id);
        setCronExpression(schedule.cron);
      } else {
        setScheduleId('');
        setSourceType('file');
        setRecipeSourcePath('');
        setDeepLinkInput('');
        setParsedRecipe(null);
        setCronExpression('0 0 14 * * *');
        setInternalValidationError(null);
        if (initialDeepLink) {
          setSourceType('deeplink');
          handleDeepLinkChange(initialDeepLink);
        }
      }
    }
  }, [isOpen, schedule, initialDeepLink, handleDeepLinkChange]);

  const handleBrowseFile = async () => {
    const defaultPath = getStorageDirectory(true);
    const filePath = await window.electron.selectFileOrDirectory(defaultPath);
    if (filePath) {
      if (filePath.endsWith('.yaml') || filePath.endsWith('.yml')) {
        setRecipeSourcePath(filePath);
        setInternalValidationError(null);
      } else {
        setInternalValidationError('Invalid file type: Please select a YAML file (.yaml or .yml)');
      }
    }
  };

  const handleLocalSubmit = async (event: FormEvent) => {
    event.preventDefault();
    setInternalValidationError(null);

    if (isEditMode) {
      await onSubmit(cronExpression);
      return;
    }

    if (!scheduleId.trim()) {
      setInternalValidationError('Schedule ID is required.');
      return;
    }

    let finalRecipeSource = '';

    if (sourceType === 'file') {
      if (!recipeSourcePath) {
        setInternalValidationError('Recipe source file is required.');
        return;
      }
      finalRecipeSource = recipeSourcePath;
    } else if (sourceType === 'deeplink') {
      if (!deepLinkInput.trim()) {
        setInternalValidationError('Deep link is required.');
        return;
      }
      if (!parsedRecipe) {
        setInternalValidationError('Invalid deep link. Please check the format.');
        return;
      }

      try {
        const yamlContent = recipeToYaml(parsedRecipe);
        const tempFileName = `schedule-${scheduleId}-${Date.now()}.yaml`;
        const tempDir = window.electron.getConfig().GOOSE_WORKING_DIR || '.';
        const tempFilePath = `${tempDir}/${tempFileName}`;

        const writeSuccess = await window.electron.writeFile(tempFilePath, yamlContent);
        if (!writeSuccess) {
          setInternalValidationError('Failed to create temporary recipe file.');
          return;
        }

        finalRecipeSource = tempFilePath;
      } catch (error) {
        console.error('Failed to convert recipe to YAML:', error);
        setInternalValidationError('Failed to process the recipe from deep link.');
        return;
      }
    }

    const newSchedulePayload: NewSchedulePayload = {
      id: scheduleId.trim(),
      recipe_source: finalRecipeSource,
      cron: cronExpression,
    };

    await onSubmit(newSchedulePayload);
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 z-40 flex items-center justify-center p-4">
      <Card className="w-full max-w-md bg-background-default shadow-xl rounded-3xl z-50 flex flex-col max-h-[90vh] overflow-hidden">
        <div className="px-8 pt-6 pb-4 flex-shrink-0">
          <div className="flex items-center gap-3">
            <img src={ClockIcon} alt="Clock" className="w-8 h-8" />
            <div className="flex-1">
              <h2 className="text-base font-semibold text-text-prominent">
                {isEditMode ? 'Edit Schedule' : 'Create New Schedule'}
              </h2>
              {isEditMode && <p className="text-sm text-text-subtle">{schedule.id}</p>}
            </div>
          </div>
        </div>

        <form
          id="schedule-form"
          onSubmit={handleLocalSubmit}
          className="px-8 py-4 space-y-4 flex-grow overflow-y-auto"
        >
          {apiErrorExternally && (
            <p className="text-text-error text-sm mb-3 p-2 bg-background-error border border-border-error rounded-md">
              {apiErrorExternally}
            </p>
          )}
          {internalValidationError && (
            <p className="text-text-error text-sm mb-3 p-2 bg-background-error border border-border-error rounded-md">
              {internalValidationError}
            </p>
          )}

          {!isEditMode && (
            <>
              <div>
                <label htmlFor="scheduleId-modal" className={modalLabelClassName}>
                  Name:
                </label>
                <Input
                  type="text"
                  id="scheduleId-modal"
                  value={scheduleId}
                  onChange={(e) => setScheduleId(e.target.value)}
                  placeholder="e.g., daily-summary-job"
                  required
                />
              </div>

              <div>
                <label className={modalLabelClassName}>Source:</label>
                <div className="space-y-2">
                  <div className="flex bg-gray-100 dark:bg-gray-700 rounded-full p-1">
                    <button
                      type="button"
                      onClick={() => setSourceType('file')}
                      className={`flex-1 px-4 py-2 text-sm font-medium rounded-full transition-all ${
                        sourceType === 'file'
                          ? 'bg-white dark:bg-gray-800 text-gray-900 dark:text-white shadow-sm'
                          : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white'
                      }`}
                    >
                      YAML
                    </button>
                    <button
                      type="button"
                      onClick={() => setSourceType('deeplink')}
                      className={`flex-1 px-4 py-2 text-sm font-medium rounded-full transition-all ${
                        sourceType === 'deeplink'
                          ? 'bg-white dark:bg-gray-800 text-gray-900 dark:text-white shadow-sm'
                          : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white'
                      }`}
                    >
                      Deep link
                    </button>
                  </div>

                  {sourceType === 'file' && (
                    <div>
                      <Button
                        type="button"
                        variant="outline"
                        onClick={handleBrowseFile}
                        className="w-full justify-center rounded-full"
                      >
                        Browse for YAML file...
                      </Button>
                      {recipeSourcePath && (
                        <p className="mt-2 text-xs text-gray-500 dark:text-gray-400 italic">
                          Selected: {recipeSourcePath}
                        </p>
                      )}
                    </div>
                  )}

                  {sourceType === 'deeplink' && (
                    <div>
                      <Input
                        type="text"
                        value={deepLinkInput}
                        onChange={(e) => handleDeepLinkChange(e.target.value)}
                        placeholder="Paste goose://bot or goose://recipe link here..."
                        className="rounded-full"
                      />
                      {parsedRecipe && (
                        <div className="mt-2 p-2 bg-green-100 dark:bg-green-900/30 rounded-md border border-green-500/50">
                          <p className="text-xs text-green-700 dark:text-green-300 font-medium">
                            ✓ Recipe parsed successfully
                          </p>
                          <p className="text-xs text-green-600 dark:text-green-400">
                            Title: {parsedRecipe.title}
                          </p>
                          <p className="text-xs text-green-600 dark:text-green-400">
                            Description: {parsedRecipe.description}
                          </p>
                        </div>
                      )}
                    </div>
                  )}
                </div>
              </div>
            </>
          )}

          <div>
            <label className={modalLabelClassName}>Schedule:</label>
            <CronPicker schedule={schedule} onChange={setCronExpression} isValid={setIsValid} />
          </div>
        </form>

        <div className="flex gap-2 px-8 py-4 border-t border-border-subtle">
          <Button
            type="button"
            variant="ghost"
            onClick={onClose}
            disabled={isLoadingExternally}
            className="flex-1 text-gray-400 hover:bg-gray-50 dark:hover:bg-gray-800"
          >
            Cancel
          </Button>
          <Button
            type="submit"
            form="schedule-form"
            disabled={isLoadingExternally || !isValid}
            className="flex-1"
          >
            {isLoadingExternally
              ? isEditMode
                ? 'Updating...'
                : 'Creating...'
              : isEditMode
                ? 'Update Schedule'
                : 'Create Schedule'}
          </Button>
        </div>
      </Card>
    </div>
  );
};
