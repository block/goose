import type React from 'react';
import { useCallback, useEffect, useState, type FormEvent } from 'react';
import ClockIcon from '@/assets/clock-icon.svg';
import { parseDeeplink, parseRecipeFromFile, type Recipe } from '@/recipe';
import { getStorageDirectory } from '@/recipe/recipe_management';
import type { ScheduledJob } from '@/schedule';
import { Button } from '@/components/atoms/button';
import { Input } from '@/components/atoms/input';
import { Card } from '@/components/molecules/ui/card';
import { CronPicker } from './CronPicker';

export interface NewSchedulePayload {
  id: string;
  recipe: Recipe;
  cron: string;
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

const modalLabelClassName = 'block text-sm font-medium text-text-default mb-1';

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

  const setScheduleIdFromTitle = useCallback((title: string) => {
    const cleanId = title
      .toLowerCase()
      .replace(/[^a-z0-9-]/g, '-')
      .replace(/-+/g, '-');
    setScheduleId(cleanId);
  }, []);

  const handleDeepLinkChange = useCallback(
    async (value: string) => {
      setDeepLinkInput(value);
      setInternalValidationError(null);

      if (value.trim()) {
        try {
          const recipe = await parseDeeplink(value.trim());
          if (!recipe) throw new Error();
          setParsedRecipe(recipe);
          if (recipe.title) {
            setScheduleIdFromTitle(recipe.title);
          }
        } catch {
          setParsedRecipe(null);
          setInternalValidationError('Invalid deep link. Please use a goose://recipe link.');
        }
      } else {
        setParsedRecipe(null);
      }
    },
    [setScheduleIdFromTitle]
  );

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

        try {
          const fileResponse = await window.electron.readFile(filePath);
          if (!fileResponse.found || fileResponse.error) {
            throw new Error('Failed to read the selected file.');
          }
          const recipe = await parseRecipeFromFile(fileResponse.file);
          if (!recipe) {
            throw new Error('Failed to parse recipe from file.');
          }
          setParsedRecipe(recipe);
          if (recipe.title) {
            setScheduleIdFromTitle(recipe.title);
          }
        } catch (e) {
          setParsedRecipe(null);
          setInternalValidationError(
            e instanceof Error ? e.message : 'Failed to parse recipe from file.'
          );
        }
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

    if (!parsedRecipe) {
      setInternalValidationError('Please provide a valid recipe source.');
      return;
    }

    const newSchedulePayload: NewSchedulePayload = {
      id: scheduleId.trim(),
      recipe: parsedRecipe,
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
              <h2 className="text-base font-semibold text-text-default">
                {isEditMode ? 'Edit Schedule' : 'Create New Schedule'}
              </h2>
              {isEditMode && <p className="text-sm text-text-muted">{schedule.id}</p>}
            </div>
          </div>
        </div>

        <form
          id="schedule-form"
          onSubmit={handleLocalSubmit}
          className="px-8 py-4 space-y-4 flex-grow overflow-y-auto"
        >
          {apiErrorExternally && (
            <p className="text-text-danger text-sm mb-3 p-2 bg-background-danger border border-border-danger rounded-md">
              {apiErrorExternally}
            </p>
          )}
          {internalValidationError && (
            <p className="text-text-danger text-sm mb-3 p-2 bg-background-danger border border-border-danger rounded-md">
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
                <div className={modalLabelClassName}>Source:</div>
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
                        placeholder="Paste goose://recipe link here..."
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
            <div className={modalLabelClassName}>Schedule:</div>
            <CronPicker schedule={schedule} onChange={setCronExpression} isValid={setIsValid} />
          </div>
        </form>

        <div className="flex gap-2 px-8 py-4 border-t border-border-default">
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
