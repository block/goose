import React, { useState, useEffect, FormEvent, useCallback } from 'react';
import { Card } from '../ui/card';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Select } from '../ui/Select';
import cronstrue from 'cronstrue';
import * as yaml from 'yaml';
import { Recipe, decodeRecipe } from '../../recipe';
import { getStorageDirectory } from '../../recipe/recipeStorage';
import ClockIcon from '../../assets/clock-icon.svg';

type FrequencyValue = 'once' | 'every' | 'daily' | 'weekly' | 'monthly';

type CustomIntervalUnit = 'minute' | 'hour' | 'day';

interface FrequencyOption {
  value: FrequencyValue;
  label: string;
}

export interface NewSchedulePayload {
  id: string;
  recipe_source: string;
  cron: string;
  execution_mode?: string;
}

interface CreateScheduleModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (payload: NewSchedulePayload) => Promise<void>;
  isLoadingExternally: boolean;
  apiErrorExternally: string | null;
}

// Interface for clean extension in YAML
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

// TODO: This 'Recipe' interface should be converted to match the OpenAPI spec for Recipe
// once we have separated the recipe from the schedule in the frontend.
// Interface for clean recipe in YAML
interface CleanRecipe {
  title: string;
  description: string;
  instructions?: string;
  prompt?: string;
  activities?: string[];
  extensions?: CleanExtension[];
  goosehints?: string;
  context?: string[];
  profile?: string;
  author?: {
    contact?: string;
    metadata?: string;
  };
  schedule?: {
    foreground: boolean;
    fallback_to_background: boolean;
    window_title?: string;
    working_directory?: string;
  };
}

const frequencies: FrequencyOption[] = [
  { value: 'once', label: 'Once' },
  { value: 'every', label: 'Every...' },
  { value: 'daily', label: 'Daily (at specific time)' },
  { value: 'weekly', label: 'Weekly (at specific time/days)' },
  { value: 'monthly', label: 'Monthly (at specific time/day)' },
];

const customIntervalUnits: { value: CustomIntervalUnit; label: string }[] = [
  { value: 'minute', label: 'minute(s)' },
  { value: 'hour', label: 'hour(s)' },
  { value: 'day', label: 'day(s)' },
];

const daysOfWeekOptions: { value: string; label: string }[] = [
  { value: '1', label: 'Mon' },
  { value: '2', label: 'Tue' },
  { value: '3', label: 'Wed' },
  { value: '4', label: 'Thu' },
  { value: '5', label: 'Fri' },
  { value: '6', label: 'Sat' },
  { value: '0', label: 'Sun' },
];

const modalLabelClassName = 'block text-sm font-medium text-text-prominent mb-1';
const cronPreviewTextColor = 'text-xs text-text-subtle mt-1';
const cronPreviewSpecialNoteColor = 'text-xs text-text-warning mt-1';
const checkboxLabelClassName = 'flex items-center text-sm text-text-default';
const checkboxInputClassName =
  'h-4 w-4 text-accent-default border-border-subtle rounded focus:ring-accent-default mr-2';

type SourceType = 'file' | 'deeplink';
type ExecutionMode = 'background' | 'foreground';

// Function to parse deep link and extract recipe config
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

// Function to convert recipe to YAML with schedule configuration
function recipeToYaml(recipe: Recipe, executionMode: ExecutionMode): string {
  // Create a clean recipe object for YAML conversion
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
        type: 'builtin', // Default type, will be overridden below
      };

      // Handle different extension types using type assertions
      if ('type' in ext && ext.type) {
        cleanExt.type = ext.type as CleanExtension['type'];

        // Use type assertions to access properties safely
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

        // Handle frontend type separately to avoid TypeScript narrowing issues
        if ((ext.type as string) === 'frontend') {
          if (extAny.tools) {
            cleanExt.tools = extAny.tools as unknown[];
          }
          if (extAny.instructions) {
            cleanExt.instructions = extAny.instructions as string;
          }
        }
      } else {
        // Fallback: try to infer type from available fields
        const extAny = ext as Record<string, unknown>;

        if (extAny.cmd) {
          cleanExt.type = 'stdio';
          cleanExt.cmd = extAny.cmd as string;
          if (extAny.args) {
            cleanExt.args = extAny.args as string[];
          }
        } else if (extAny.command) {
          // Handle legacy 'command' field by converting to 'cmd'
          cleanExt.type = 'stdio';
          cleanExt.cmd = extAny.command as string;
        } else if (extAny.uri) {
          // Default to streamable_http for URI-based extensions for forward compatibility
          cleanExt.type = 'streamable_http';
          cleanExt.uri = extAny.uri as string;
        } else if (extAny.tools) {
          cleanExt.type = 'frontend';
          cleanExt.tools = extAny.tools as unknown[];
          if (extAny.instructions) {
            cleanExt.instructions = extAny.instructions as string;
          }
        } else {
          // Default to builtin if we can't determine type
          cleanExt.type = 'builtin';
        }
      }

      // Add common optional fields
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

  if (recipe.goosehints) {
    cleanRecipe.goosehints = recipe.goosehints;
  }

  if (recipe.context && recipe.context.length > 0) {
    cleanRecipe.context = recipe.context;
  }

  if (recipe.profile) {
    cleanRecipe.profile = recipe.profile;
  }

  if (recipe.author) {
    cleanRecipe.author = {
      contact: recipe.author.contact || undefined,
      metadata: recipe.author.metadata || undefined,
    };
  }

  // Add schedule configuration based on execution mode
  cleanRecipe.schedule = {
    foreground: executionMode === 'foreground',
    fallback_to_background: true, // Always allow fallback
    window_title: executionMode === 'foreground' ? `${recipe.title} - Scheduled` : undefined,
  };

  return yaml.stringify(cleanRecipe);
}

export const CreateScheduleModal: React.FC<CreateScheduleModalProps> = ({
  isOpen,
  onClose,
  onSubmit,
  isLoadingExternally,
  apiErrorExternally,
}) => {
  const [scheduleId, setScheduleId] = useState<string>('');
  const [sourceType, setSourceType] = useState<SourceType>('file');
  const [executionMode, setExecutionMode] = useState<ExecutionMode>('background');
  const [recipeSourcePath, setRecipeSourcePath] = useState<string>('');
  const [deepLinkInput, setDeepLinkInput] = useState<string>('');
  const [parsedRecipe, setParsedRecipe] = useState<Recipe | null>(null);
  const [frequency, setFrequency] = useState<FrequencyValue>('daily');
  const [customIntervalValue, setCustomIntervalValue] = useState<number>(1);
  const [customIntervalUnit, setCustomIntervalUnit] = useState<CustomIntervalUnit>('minute');
  const [selectedDate, setSelectedDate] = useState<string>(
    () => new Date().toISOString().split('T')[0]
  );
  const [selectedTime, setSelectedTime] = useState<string>('09:00');
  const [selectedMinute, setSelectedMinute] = useState<string>('0');
  const [selectedDaysOfWeek, setSelectedDaysOfWeek] = useState<Set<string>>(new Set(['1']));
  const [selectedDayOfMonth, setSelectedDayOfMonth] = useState<string>('1');
  const [derivedCronExpression, setDerivedCronExpression] = useState<string>('');
  const [readableCronExpression, setReadableCronExpression] = useState<string>('');
  const [internalValidationError, setInternalValidationError] = useState<string | null>(null);

  const handleDeepLinkChange = useCallback(
    async (value: string) => {
      setDeepLinkInput(value);
      setInternalValidationError(null);

      if (value.trim()) {
        try {
          const recipe = await parseDeepLink(value.trim());
          if (recipe) {
            setParsedRecipe(recipe);
            // Auto-populate schedule ID from recipe title if available
            if (recipe.title && !scheduleId) {
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
        } catch (error) {
          setParsedRecipe(null);
          setInternalValidationError(
            'Failed to parse deep link. Please ensure using a goose://bot or goose://recipe link and try again.'
          );
        }
      } else {
        setParsedRecipe(null);
      }
    },
    [scheduleId]
  );

  useEffect(() => {
    // Check for pending deep link when modal opens
    if (isOpen) {
      const pendingDeepLink = localStorage.getItem('pendingScheduleDeepLink');
      if (pendingDeepLink) {
        localStorage.removeItem('pendingScheduleDeepLink');
        setSourceType('deeplink');
        handleDeepLinkChange(pendingDeepLink);
      }
    }
  }, [isOpen, handleDeepLinkChange]);

  const resetForm = () => {
    setScheduleId('');
    setSourceType('file');
    setExecutionMode('background');
    setRecipeSourcePath('');
    setDeepLinkInput('');
    setParsedRecipe(null);
    setFrequency('daily');
    setCustomIntervalValue(1);
    setCustomIntervalUnit('minute');
    setSelectedDate(new Date().toISOString().split('T')[0]);
    setSelectedTime('09:00');
    setSelectedMinute('0');
    setSelectedDaysOfWeek(new Set(['1']));
    setSelectedDayOfMonth('1');
    setInternalValidationError(null);
    setReadableCronExpression('');
  };

  const handleBrowseFile = async () => {
    // Default to global recipes directory, but fallback to local if needed
    const defaultPath = getStorageDirectory(true);
    const filePath = await window.electron.selectFileOrDirectory(defaultPath);
    if (filePath) {
      if (filePath.endsWith('.yaml') || filePath.endsWith('.yml')) {
        setRecipeSourcePath(filePath);
        setInternalValidationError(null);
      } else {
        setInternalValidationError('Invalid file type: Please select a YAML file (.yaml or .yml)');
        console.warn('Invalid file type: Please select a YAML file (.yaml or .yml)');
      }
    }
  };

  useEffect(() => {
    const generateCronExpression = (): string => {
      const timeParts = selectedTime.split(':');
      const minutePart = timeParts.length > 1 ? String(parseInt(timeParts[1], 10)) : '0';
      const hourPart = timeParts.length > 0 ? String(parseInt(timeParts[0], 10)) : '0';
      if (isNaN(parseInt(minutePart)) || isNaN(parseInt(hourPart))) {
        return 'Invalid time format.';
      }

      // Temporal uses 5-field cron: minute hour day month dayofweek (no seconds)
      switch (frequency) {
        case 'once':
          if (selectedDate && selectedTime) {
            try {
              const dateObj = new Date(`${selectedDate}T${selectedTime}`);
              if (isNaN(dateObj.getTime())) return "Invalid date/time for 'once'.";
              return `${dateObj.getMinutes()} ${dateObj.getHours()} ${dateObj.getDate()} ${
                dateObj.getMonth() + 1
              } *`;
            } catch (e) {
              return "Error parsing date/time for 'once'.";
            }
          }
          return 'Date and Time are required for "Once" frequency.';
        case 'every': {
          if (customIntervalValue <= 0) {
            return 'Custom interval value must be greater than 0.';
          }
          switch (customIntervalUnit) {
            case 'minute':
              return `*/${customIntervalValue} * * * *`;
            case 'hour':
              return `0 */${customIntervalValue} * * *`;
            case 'day':
              return `0 0 */${customIntervalValue} * *`;
            default:
              return 'Invalid custom interval unit.';
          }
        }
        case 'daily':
          return `${minutePart} ${hourPart} * * *`;
        case 'weekly': {
          if (selectedDaysOfWeek.size === 0) {
            return 'Select at least one day for weekly frequency.';
          }
          const days = Array.from(selectedDaysOfWeek)
            .sort((a, b) => parseInt(a) - parseInt(b))
            .join(',');
          return `${minutePart} ${hourPart} * * ${days}`;
        }
        case 'monthly': {
          const sDayOfMonth = parseInt(selectedDayOfMonth, 10);
          if (isNaN(sDayOfMonth) || sDayOfMonth < 1 || sDayOfMonth > 31) {
            return 'Invalid day of month (1-31) for monthly frequency.';
          }
          return `${minutePart} ${hourPart} ${sDayOfMonth} * *`;
        }
        default:
          return 'Invalid frequency selected.';
      }
    };
    const cron = generateCronExpression();
    setDerivedCronExpression(cron);
    try {
      if (
        cron.includes('Invalid') ||
        cron.includes('required') ||
        cron.includes('Error') ||
        cron.includes('Select at least one')
      ) {
        setReadableCronExpression('Invalid cron details provided.');
      } else {
        setReadableCronExpression(cronstrue.toString(cron));
      }
    } catch (e) {
      setReadableCronExpression('Could not parse cron string.');
    }
  }, [
    frequency,
    customIntervalValue,
    customIntervalUnit,
    selectedDate,
    selectedTime,
    selectedMinute,
    selectedDaysOfWeek,
    selectedDayOfMonth,
  ]);

  const handleDayOfWeekChange = (dayValue: string) => {
    setSelectedDaysOfWeek((prev) => {
      const newSet = new Set(prev);
      if (newSet.has(dayValue)) {
        newSet.delete(dayValue);
      } else {
        newSet.add(dayValue);
      }
      return newSet;
    });
  };

  const handleLocalSubmit = async (event: FormEvent) => {
    event.preventDefault();
    setInternalValidationError(null);

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
        // Convert recipe to YAML and save to a temporary file
        const yamlContent = recipeToYaml(parsedRecipe, executionMode);
        console.log('Generated YAML content:', yamlContent); // Debug log
        const tempFileName = `schedule-${scheduleId}-${Date.now()}.yaml`;
        const tempDir = window.electron.getConfig().GOOSE_WORKING_DIR || '.';
        const tempFilePath = `${tempDir}/${tempFileName}`;

        // Write the YAML file
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

    if (
      !derivedCronExpression ||
      derivedCronExpression.includes('Invalid') ||
      derivedCronExpression.includes('required') ||
      derivedCronExpression.includes('Error') ||
      derivedCronExpression.includes('Select at least one')
    ) {
      setInternalValidationError(`Invalid cron expression: ${derivedCronExpression}`);
      return;
    }
    if (frequency === 'weekly' && selectedDaysOfWeek.size === 0) {
      setInternalValidationError('For weekly frequency, select at least one day.');
      return;
    }

    const newSchedulePayload: NewSchedulePayload = {
      id: scheduleId.trim(),
      recipe_source: finalRecipeSource,
      cron: derivedCronExpression,
      execution_mode: executionMode,
    };

    await onSubmit(newSchedulePayload);
  };

  const handleClose = () => {
    resetForm();
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 z-40 flex items-center justify-center p-4">
      <Card className="w-full max-w-md bg-background-default shadow-xl rounded-3xl z-50 flex flex-col max-h-[90vh] overflow-hidden">
        <div className="px-8 pt-8 pb-4 flex-shrink-0 text-center">
          <div className="flex flex-col items-center">
            <img src={ClockIcon} alt="Clock" className="w-11 h-11 mb-2" />
            <h2 className="text-base font-semibold text-text-prominent">Create New Schedule</h2>
            <p className="text-base text-text-subtle mt-2 max-w-sm">
              Create a new schedule using the settings below to do things like automatically run
              tasks or create files
            </p>
          </div>
        </div>

        <form
          id="new-schedule-form"
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
                  {executionMode === 'foreground' && (
                    <div className="mt-2 p-2 bg-blue-50 dark:bg-blue-900/20 rounded-md border border-blue-200 dark:border-blue-800">
                      <p className="text-xs text-blue-700 dark:text-blue-300">
                        <strong>Note:</strong> For foreground execution with YAML files, add this to
                        your recipe:
                      </p>
                      <pre className="text-xs text-blue-600 dark:text-blue-400 mt-1 font-mono bg-blue-100 dark:bg-blue-900/40 p-1 rounded">
                        {`schedule:
  foreground: true
  fallback_to_background: true`}
                      </pre>
                    </div>
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

          <div>
            <label className={modalLabelClassName}>Execution Mode:</label>
            <div className="space-y-2">
              <div className="flex bg-gray-100 dark:bg-gray-700 rounded-full p-1">
                <button
                  type="button"
                  onClick={() => setExecutionMode('background')}
                  className={`flex-1 px-4 py-2 text-sm font-medium rounded-full transition-all ${
                    executionMode === 'background'
                      ? 'bg-white dark:bg-gray-800 text-gray-900 dark:text-white shadow-sm'
                      : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white'
                  }`}
                >
                  Background
                </button>
                <button
                  type="button"
                  onClick={() => setExecutionMode('foreground')}
                  className={`flex-1 px-4 py-2 text-sm font-medium rounded-full transition-all ${
                    executionMode === 'foreground'
                      ? 'bg-white dark:bg-gray-800 text-gray-900 dark:text-white shadow-sm'
                      : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white'
                  }`}
                >
                  Foreground
                </button>
              </div>

              <div className="text-xs text-gray-500 dark:text-gray-400 px-2">
                {executionMode === 'background' ? (
                  <p>
                    <strong>Background:</strong> Runs silently in the background without opening a
                    window. Results are saved to session storage.
                  </p>
                ) : (
                  <p>
                    <strong>Foreground:</strong> Opens in a desktop window when the Goose app is
                    running. Falls back to background if the app is not available.
                  </p>
                )}
              </div>
            </div>
          </div>

          <div>
            <label htmlFor="frequency-modal" className={modalLabelClassName}>
              Frequency:
            </label>
            <Select
              instanceId="frequency-select-modal"
              options={frequencies}
              value={frequencies.find((f) => f.value === frequency)}
              onChange={(newValue: unknown) => {
                const selectedOption = newValue as FrequencyOption | null;
                if (selectedOption) setFrequency(selectedOption.value);
              }}
              placeholder="Select frequency..."
            />
          </div>

          {frequency === 'every' && (
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label htmlFor="customIntervalValue-modal" className={modalLabelClassName}>
                  Every:
                </label>
                <Input
                  type="number"
                  id="customIntervalValue-modal"
                  min="1"
                  max="999"
                  value={customIntervalValue}
                  onChange={(e) => setCustomIntervalValue(parseInt(e.target.value) || 1)}
                  required
                />
              </div>
              <div>
                <label htmlFor="customIntervalUnit-modal" className={modalLabelClassName}>
                  Unit:
                </label>
                <Select
                  instanceId="custom-interval-unit-select-modal"
                  options={customIntervalUnits}
                  value={customIntervalUnits.find((u) => u.value === customIntervalUnit)}
                  onChange={(newValue: unknown) => {
                    const selectedUnit = newValue as {
                      value: CustomIntervalUnit;
                      label: string;
                    } | null;
                    if (selectedUnit) setCustomIntervalUnit(selectedUnit.value);
                  }}
                  placeholder="Select unit..."
                />
              </div>
            </div>
          )}

          {frequency === 'once' && (
            <>
              <div>
                <label htmlFor="onceDate-modal" className={modalLabelClassName}>
                  Date:
                </label>
                <Input
                  type="date"
                  id="onceDate-modal"
                  value={selectedDate}
                  onChange={(e) => setSelectedDate(e.target.value)}
                  required
                />
              </div>
              <div>
                <label htmlFor="onceTime-modal" className={modalLabelClassName}>
                  Time:
                </label>
                <Input
                  type="time"
                  id="onceTime-modal"
                  value={selectedTime}
                  onChange={(e) => setSelectedTime(e.target.value)}
                  required
                />
              </div>
            </>
          )}
          {(frequency === 'daily' || frequency === 'weekly' || frequency === 'monthly') && (
            <div>
              <label htmlFor="commonTime-modal" className={modalLabelClassName}>
                Time:
              </label>
              <Input
                type="time"
                id="commonTime-modal"
                value={selectedTime}
                onChange={(e) => setSelectedTime(e.target.value)}
                required
              />
            </div>
          )}
          {frequency === 'weekly' && (
            <div>
              <label className={modalLabelClassName}>Days of Week:</label>
              <div className="grid grid-cols-3 sm:grid-cols-4 gap-2 mt-1">
                {daysOfWeekOptions.map((day) => (
                  <label key={day.value} className={checkboxLabelClassName}>
                    <input
                      type="checkbox"
                      value={day.value}
                      checked={selectedDaysOfWeek.has(day.value)}
                      onChange={() => handleDayOfWeekChange(day.value)}
                      className={checkboxInputClassName}
                    />
                    {day.label}
                  </label>
                ))}
              </div>
            </div>
          )}
          {frequency === 'monthly' && (
            <div>
              <label htmlFor="monthlyDay-modal" className={modalLabelClassName}>
                Day of Month (1-31):
              </label>
              <Input
                type="number"
                id="monthlyDay-modal"
                min="1"
                max="31"
                value={selectedDayOfMonth}
                onChange={(e) => setSelectedDayOfMonth(e.target.value)}
                required
              />
            </div>
          )}
          <div className="mt-4 p-3 bg-gray-100 dark:bg-gray-700/50 rounded-md border border-gray-200 dark:border-gray-600">
            <p className="text-sm font-medium text-gray-700 dark:text-gray-300">
              Generated Cron:{' '}
              <code className="text-xs bg-gray-200 dark:bg-gray-600 p-1 rounded">
                {derivedCronExpression}
              </code>
            </p>
            <p className={`${cronPreviewTextColor} mt-2`}>
              <b>Human Readable:</b> {readableCronExpression}
            </p>
            <p className={cronPreviewTextColor}>
              Syntax: M H D M DoW (M=minute, H=hour, D=day, M=month, DoW=day of week: 0/7=Sun)
            </p>
            {frequency === 'once' && (
              <p className={cronPreviewSpecialNoteColor}>
                Note: "Once" schedules recur annually. True one-time tasks may need backend deletion
                after execution.
              </p>
            )}
          </div>
        </form>

        {/* Actions */}
        <div className="mt-[8px] ml-[-24px] mr-[-24px] pt-[16px]">
          <Button
            type="submit"
            form="new-schedule-form"
            variant="ghost"
            disabled={isLoadingExternally}
            className="w-full h-[60px] rounded-none border-t text-gray-900 dark:text-white hover:bg-gray-50 dark:border-gray-600 text-lg font-medium"
          >
            {isLoadingExternally ? 'Creating...' : 'Create Schedule'}
          </Button>
          <Button
            type="button"
            variant="ghost"
            onClick={handleClose}
            disabled={isLoadingExternally}
            className="w-full h-[60px] rounded-none border-t text-gray-400 hover:bg-gray-50 dark:border-gray-600 text-lg font-regular"
          >
            Cancel
          </Button>
        </div>
      </Card>
    </div>
  );
};
