import { useState, useEffect } from 'react';
import { useForm } from '@tanstack/react-form';
import { z } from 'zod';
import { FileText, Clock, ChevronDown, ChevronUp, AlertCircle } from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';
import { Button } from '../ui/button';
import { Recipe } from '../../recipe';
import { saveRecipe, listSavedRecipes } from '../../recipe/recipeStorage';
import { toastSuccess, toastError } from '../../toasts';
import { useEscapeKey } from '../../hooks/useEscapeKey';
import { RecipeNameField, recipeNameSchema } from './shared/RecipeNameField';
import { generateRecipeNameFromTitle } from './shared/recipeNameUtils';
import { validateJsonSchema, getValidationErrorMessages } from '../../recipe/validation';
import { createSchedule } from '../../schedule';
import cronstrue from 'cronstrue';

interface CreateRecipeFormProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
}

// Define Zod schema for the entire form
const createRecipeSchema = z.object({
  title: z.string().min(3, 'Title must be at least 3 characters'),
  description: z.string().min(10, 'Description must be at least 10 characters'),
  instructions: z.string().min(20, 'Instructions must be at least 20 characters'),
  prompt: z.string(),
  activities: z.string(),
  jsonSchema: z.string(),
  recipeName: recipeNameSchema,
  global: z.boolean(),
  enableSchedule: z.boolean(),
  cronExpression: z.string(),
  scheduleId: z.string(),
});

export default function CreateRecipeForm({ isOpen, onClose, onSuccess }: CreateRecipeFormProps) {
  const [creating, setCreating] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [cronPreview, setCronPreview] = useState<string>('');
  const [existingRecipes, setExistingRecipes] = useState<string[]>([]);
  const [recipeNameWarning, setRecipeNameWarning] = useState<string>('');

  // Handle Esc key for modal
  useEscapeKey(isOpen, onClose);

  // Load existing recipe names when modal opens
  useEffect(() => {
    if (isOpen) {
      listSavedRecipes().then((recipes) => {
        setExistingRecipes(recipes.map((r) => r.name));
      }).catch((err) => {
        console.error('Failed to load existing recipes:', err);
      });
    }
  }, [isOpen]);

  // Check for duplicate recipe names
  const checkForDuplicate = (recipeName: string): string => {
    if (!recipeName) return recipeName;
    
    let finalName = recipeName;
    let counter = 1;
    
    while (existingRecipes.includes(finalName)) {
      finalName = `${recipeName}-${counter}`;
      counter++;
    }
    
    if (finalName !== recipeName) {
      setRecipeNameWarning(`Recipe name "${recipeName}" already exists. Using "${finalName}" instead.`);
    } else {
      setRecipeNameWarning('');
    }
    
    return finalName;
  };

  const createRecipeForm = useForm({
    defaultValues: {
      title: '',
      description: '',
      instructions: '',
      prompt: '',
      activities: '',
      jsonSchema: '',
      recipeName: '',
      global: true,
      enableSchedule: false,
      cronExpression: '0 0 * * *', // Daily at midnight
      scheduleId: '',
    },
    validators: {
      onSubmit: createRecipeSchema,
    },
    onSubmit: async ({ value }) => {
      setCreating(true);
      try {
        // Parse activities from comma-separated string
        const activities = value.activities
          .split(',')
          .map((activity) => activity.trim())
          .filter((activity) => activity.length > 0);

        // Parse and validate JSON schema if provided
        let jsonSchemaObj = undefined;
        if (value.jsonSchema && value.jsonSchema.trim()) {
          try {
            jsonSchemaObj = JSON.parse(value.jsonSchema.trim());
            // Validate the JSON schema syntax
            const validationResult = validateJsonSchema(jsonSchemaObj);
            if (!validationResult.success) {
              const errorMessages = getValidationErrorMessages(validationResult.errors);
              throw new Error(`Invalid JSON schema: ${errorMessages.join(', ')}`);
            }
          } catch (error) {
            throw new Error(
              `JSON Schema parsing error: ${error instanceof Error ? error.message : 'Invalid JSON'}`
            );
          }
        }

        // Create the recipe object
        const recipe: Recipe = {
          title: value.title.trim(),
          description: value.description.trim(),
          instructions: value.instructions.trim(),
          prompt: value.prompt.trim() || undefined,
          activities: activities.length > 0 ? activities : undefined,
          response: jsonSchemaObj ? { json_schema: jsonSchemaObj } : undefined,
        };

        const savedRecipePath = await saveRecipe(recipe, {
          name: value.recipeName.trim(),
          global: value.global,
        });

        // Create schedule if enabled
        if (value.enableSchedule && value.cronExpression && value.scheduleId) {
          try {
            // Get the recipe path - construct it based on global/local
            const recipeSource = value.global 
              ? `~/.config/goose/recipes/${value.recipeName.trim()}.yaml`
              : `.goose/recipes/${value.recipeName.trim()}.yaml`;
            
            await createSchedule({
              id: value.scheduleId.trim(),
              recipe_source: recipeSource,
              cron: value.cronExpression.trim(),
            });
            
            toastSuccess({
              title: value.recipeName.trim(),
              msg: 'Recipe and schedule created successfully',
            });
          } catch (scheduleError) {
            console.error('Failed to create schedule:', scheduleError);
            toastError({
              title: 'Schedule Creation Failed',
              msg: 'Recipe was created but schedule failed. You can create the schedule from the Scheduler view.',
              traceback: scheduleError instanceof Error ? scheduleError.message : String(scheduleError),
            });
          }
        } else {
          toastSuccess({
            title: value.recipeName.trim(),
            msg: 'Recipe created successfully',
          });
        }

        // Reset dialog state
        createRecipeForm.reset({
          title: '',
          description: '',
          instructions: '',
          prompt: '',
          activities: '',
          jsonSchema: '',
          recipeName: '',
          global: true,
          enableSchedule: false,
          cronExpression: '0 0 * * *',
          scheduleId: '',
        });
        setCronPreview('');
        onClose();

        onSuccess();
      } catch (error) {
        console.error('Failed to create recipe:', error);

        toastError({
          title: 'Create Failed',
          msg: `Failed to create recipe: ${error instanceof Error ? error.message : 'Unknown error'}`,
          traceback: error instanceof Error ? error.message : String(error),
        });
      } finally {
        setCreating(false);
      }
    },
  });

  // Clear form when modal opens
  useEffect(() => {
    if (isOpen) {
      // Start with empty fields
      createRecipeForm.reset({
        title: '',
        description: '',
        instructions: '',
        prompt: '',
        activities: '',
        jsonSchema: '',
        recipeName: '',
        global: true,
        enableSchedule: false,
        cronExpression: '0 0 * * *',
        scheduleId: '',
      });
      setRecipeNameWarning('');
    }
  }, [isOpen]);

  const handleClose = () => {
    // Reset form to default values
    createRecipeForm.reset({
      title: '',
      description: '',
      instructions: '',
      prompt: '',
      activities: '',
      jsonSchema: '',
      recipeName: '',
      global: true,
      enableSchedule: false,
      cronExpression: '0 0 * * *',
      scheduleId: '',
    });
    setCronPreview('');
    setRecipeNameWarning('');
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-[300] flex items-center justify-center bg-black/50">
      <div className="bg-background-default border border-border-subtle rounded-lg p-6 w-[700px] max-w-[90vw] max-h-[90vh] overflow-y-auto">
        <h3 className="text-lg font-medium text-text-standard mb-4">Create New Recipe</h3>

        <form
          onSubmit={(e) => {
            e.preventDefault();
            e.stopPropagation();
            createRecipeForm.handleSubmit();
          }}
        >
          <div className="space-y-4">
            <createRecipeForm.Field name="title">
              {(field) => (
                <div>
                  <label
                    htmlFor="create-title"
                    className="block text-sm font-medium text-text-standard mb-2"
                  >
                    Title <span className="text-red-500">*</span>
                  </label>
                  <input
                    id="create-title"
                    type="text"
                    value={field.state.value}
                    onChange={(e) => {
                      const value = e.target.value;
                      field.handleChange(value);
                      // Auto-generate recipe name when title changes and check for duplicates
                      if (value.trim()) {
                        const suggestedName = generateRecipeNameFromTitle(value);
                        const finalName = checkForDuplicate(suggestedName);
                        createRecipeForm.setFieldValue('recipeName', finalName);
                      } else {
                        createRecipeForm.setFieldValue('recipeName', '');
                        setRecipeNameWarning('');
                      }
                    }}
                    onBlur={field.handleBlur}
                    className={`w-full p-3 border rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 placeholder:text-text-muted/50 ${
                      field.state.meta.errors.length > 0 ? 'border-red-500' : 'border-border-subtle'
                    }`}
                    placeholder="Recipe title"
                    autoFocus
                  />
                  {field.state.meta.errors.length > 0 && (
                    <p className="text-red-500 text-sm mt-1">
                      {typeof field.state.meta.errors[0] === 'string'
                        ? field.state.meta.errors[0]
                        : field.state.meta.errors[0]?.message || String(field.state.meta.errors[0])}
                    </p>
                  )}
                  {recipeNameWarning && (
                    <div className="flex items-start gap-2 mt-2 p-2 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-lg">
                      <AlertCircle className="w-4 h-4 text-yellow-600 dark:text-yellow-500 mt-0.5 flex-shrink-0" />
                      <p className="text-xs text-yellow-800 dark:text-yellow-200">{recipeNameWarning}</p>
                    </div>
                  )}
                </div>
              )}
            </createRecipeForm.Field>

            <createRecipeForm.Field name="description">
              {(field) => (
                <div>
                  <label
                    htmlFor="create-description"
                    className="block text-sm font-medium text-text-standard mb-2"
                  >
                    Description <span className="text-red-500">*</span>
                  </label>
                  <input
                    id="create-description"
                    type="text"
                    value={field.state.value}
                    onChange={(e) => field.handleChange(e.target.value)}
                    onBlur={field.handleBlur}
                    className={`w-full p-3 border rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 placeholder:text-text-muted/50 ${
                      field.state.meta.errors.length > 0 ? 'border-red-500' : 'border-border-subtle'
                    }`}
                    placeholder="Brief description of what this recipe does"
                  />
                  {field.state.meta.errors.length > 0 && (
                    <p className="text-red-500 text-sm mt-1">
                      {typeof field.state.meta.errors[0] === 'string'
                        ? field.state.meta.errors[0]
                        : field.state.meta.errors[0]?.message || String(field.state.meta.errors[0])}
                    </p>
                  )}
                </div>
              )}
            </createRecipeForm.Field>

            <createRecipeForm.Field name="instructions">
              {(field) => (
                <div>
                  <label
                    htmlFor="create-instructions"
                    className="block text-sm font-medium text-text-standard mb-2"
                  >
                    Instructions <span className="text-red-500">*</span>
                  </label>
                  <textarea
                    id="create-instructions"
                    value={field.state.value}
                    onChange={(e) => field.handleChange(e.target.value)}
                    onBlur={field.handleBlur}
                    className={`w-full p-3 border rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none font-mono text-sm placeholder:text-text-muted/50 ${
                      field.state.meta.errors.length > 0 ? 'border-red-500' : 'border-border-subtle'
                    }`}
                    placeholder="Detailed instructions for the AI agent..."
                    rows={8}
                  />
                  {field.state.value && (
                    <p className="text-xs text-text-muted mt-1">
                      Use {`{{parameter_name}}`} to define parameters that users can fill in
                    </p>
                  )}
                  {field.state.meta.errors.length > 0 && (
                    <p className="text-red-500 text-sm mt-1">
                      {typeof field.state.meta.errors[0] === 'string'
                        ? field.state.meta.errors[0]
                        : field.state.meta.errors[0]?.message || String(field.state.meta.errors[0])}
                    </p>
                  )}
                </div>
              )}
            </createRecipeForm.Field>

            {/* Recipe name is auto-generated from title - hidden from UI but kept in form state */}
            <createRecipeForm.Field name="recipeName">
              {(field) => (
                <input
                  type="hidden"
                  value={field.state.value}
                  onChange={(e) => field.handleChange(e.target.value)}
                />
              )}
            </createRecipeForm.Field>

            {/* Schedule Section */}
            <createRecipeForm.Field name="enableSchedule">
              {(field) => (
                <div className="border border-border-subtle rounded-lg">
                  <label className="flex items-center justify-between cursor-pointer p-4">
                    <div className="flex items-center gap-2">
                      <Clock className="w-4 h-4 text-blue-500" />
                      <span className="text-sm font-medium text-text-standard">
                        Schedule this recipe
                      </span>
                    </div>
                    <button
                      type="button"
                      role="switch"
                      aria-checked={field.state.value}
                      onClick={() => {
                        const newValue = !field.state.value;
                        field.handleChange(newValue);
                        if (newValue) {
                          const recipeName = createRecipeForm.getFieldValue('recipeName');
                          createRecipeForm.setFieldValue('scheduleId', recipeName || 'scheduled-recipe');
                        }
                      }}
                      className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 ${
                        field.state.value ? 'bg-blue-600' : 'bg-gray-300 dark:bg-gray-600'
                      }`}
                    >
                      <span
                        className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                          field.state.value ? 'translate-x-6' : 'translate-x-1'
                        }`}
                      />
                    </button>
                  </label>
                  
                  <AnimatePresence initial={false}>
                    {field.state.value && (
                      <motion.div
                        initial={{ height: 0, opacity: 0 }}
                        animate={{ height: 'auto', opacity: 1 }}
                        exit={{ height: 0, opacity: 0 }}
                        transition={{ duration: 0.2, ease: 'easeInOut' }}
                        className="overflow-hidden"
                      >
                        <div className="p-4 pt-0 space-y-4 border-t border-border-subtle">
                          <createRecipeForm.Field name="cronExpression">
                        {(cronField) => (
                          <div>
                            <label htmlFor="cron-expression" className="block text-sm font-medium text-text-standard mb-2">
                              Schedule (Cron Expression)
                            </label>
                            <input
                              id="cron-expression"
                              type="text"
                              value={cronField.state.value}
                              onChange={(e) => {
                                cronField.handleChange(e.target.value);
                                try {
                                  const preview = cronstrue.toString(e.target.value);
                                  setCronPreview(preview);
                                } catch (error) {
                                  setCronPreview('Invalid cron expression');
                                }
                              }}
                              onBlur={cronField.handleBlur}
                              className="w-full p-2 border border-border-subtle rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 font-mono text-sm"
                              placeholder="0 0 * * * (Daily at midnight)"
                            />
                            {cronPreview && (
                              <p className="text-xs text-blue-600 dark:text-blue-400 mt-1">
                                {cronPreview}
                              </p>
                            )}
                            <p className="text-xs text-text-muted mt-1">
                              Examples: "0 0 * * *" (daily), "0 */6 * * *" (every 6 hours), "0 9 * * 1" (Mondays at 9am)
                            </p>
                          </div>
                        )}
                      </createRecipeForm.Field>

                      <createRecipeForm.Field name="scheduleId">
                        {(idField) => (
                          <div>
                            <label htmlFor="schedule-id" className="block text-sm font-medium text-text-standard mb-2">
                              Schedule ID
                            </label>
                            <input
                              id="schedule-id"
                              type="text"
                              value={idField.state.value}
                              onChange={(e) => idField.handleChange(e.target.value)}
                              onBlur={idField.handleBlur}
                              className="w-full p-2 border border-border-subtle rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm"
                              placeholder="schedule-name"
                            />
                            {idField.state.value && (
                              <p className="text-xs text-text-muted mt-1">
                                Unique identifier for this schedule
                              </p>
                            )}
                          </div>
                        )}
                      </createRecipeForm.Field>
                        </div>
                      </motion.div>
                    )}
                  </AnimatePresence>
                </div>
              )}
            </createRecipeForm.Field>

            {/* Advanced Settings Collapsible */}
            <div className="border border-border-subtle rounded-lg">
              <button
                type="button"
                onClick={() => setShowAdvanced(!showAdvanced)}
                className="w-full flex items-center justify-between p-4 hover:bg-background-muted transition-colors rounded-lg"
              >
                <span className="text-sm font-medium text-text-standard">Advanced Settings</span>
                {showAdvanced ? (
                  <ChevronUp className="w-4 h-4 text-text-muted" />
                ) : (
                  <ChevronDown className="w-4 h-4 text-text-muted" />
                )}
              </button>
              
              <AnimatePresence initial={false}>
                {showAdvanced && (
                  <motion.div
                    initial={{ height: 0, opacity: 0 }}
                    animate={{ height: 'auto', opacity: 1 }}
                    exit={{ height: 0, opacity: 0 }}
                    transition={{ duration: 0.2, ease: 'easeInOut' }}
                    className="overflow-hidden"
                  >
                    <div className="p-4 pt-0 space-y-4 border-t border-border-subtle">
                  <createRecipeForm.Field name="prompt">
                    {(field) => (
                      <div>
                        <label
                          htmlFor="create-prompt"
                          className="block text-sm font-medium text-text-standard mb-2"
                        >
                          Initial Prompt (Optional)
                        </label>
                        <textarea
                          id="create-prompt"
                          value={field.state.value}
                          onChange={(e) => field.handleChange(e.target.value)}
                          onBlur={field.handleBlur}
                          className="w-full p-3 border border-border-subtle rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
                          placeholder="First message to send when the recipe starts..."
                          rows={3}
                        />
                      </div>
                    )}
                  </createRecipeForm.Field>

                  <createRecipeForm.Field name="activities">
                    {(field) => (
                      <div>
                        <label
                          htmlFor="create-activities"
                          className="block text-sm font-medium text-text-standard mb-2"
                        >
                          Activities (Optional)
                        </label>
                        <input
                          id="create-activities"
                          type="text"
                          value={field.state.value}
                          onChange={(e) => field.handleChange(e.target.value)}
                          onBlur={field.handleBlur}
                          className="w-full p-3 border border-border-subtle rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500"
                          placeholder="coding, debugging, testing, documentation (comma-separated)"
                        />
                        {field.state.value && (
                          <p className="text-xs text-text-muted mt-1">
                            Comma-separated list of activities this recipe helps with
                          </p>
                        )}
                      </div>
                    )}
                  </createRecipeForm.Field>

                  <createRecipeForm.Field name="jsonSchema">
                    {(field) => (
                      <div>
                        <label
                          htmlFor="create-json-schema"
                          className="block text-sm font-medium text-text-standard mb-2"
                        >
                          Response JSON Schema (Optional)
                        </label>
                        <textarea
                          id="create-json-schema"
                          value={field.state.value}
                          onChange={(e) => field.handleChange(e.target.value)}
                          onBlur={field.handleBlur}
                          className={`w-full p-3 border rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none font-mono text-sm ${
                            field.state.meta.errors.length > 0 ? 'border-red-500' : 'border-border-subtle'
                          }`}
                          placeholder={`{
  "type": "object",
  "properties": {
    "result": {
      "type": "string",
      "description": "The main result"
    }
  },
  "required": ["result"]
}`}
                          rows={6}
                        />
                        {field.state.value && (
                          <p className="text-xs text-text-muted mt-1">
                            Define the expected structure of the AI's response using JSON Schema format
                          </p>
                        )}
                        {field.state.meta.errors.length > 0 && (
                          <p className="text-red-500 text-sm mt-1">
                            {typeof field.state.meta.errors[0] === 'string'
                              ? field.state.meta.errors[0]
                              : field.state.meta.errors[0]?.message || String(field.state.meta.errors[0])}
                          </p>
                        )}
                      </div>
                    )}
                  </createRecipeForm.Field>

                  <createRecipeForm.Field name="global">
                    {(field) => (
                      <div>
                        <label className="block text-sm font-medium text-text-standard mb-2">
                          Save Location
                        </label>
                        <div className="space-y-2">
                          <label className="flex items-center">
                            <input
                              type="radio"
                              name="create-save-location"
                              checked={field.state.value === true}
                              onChange={() => field.handleChange(true)}
                              className="mr-2"
                            />
                            <span className="text-sm text-text-standard">
                              Global - Available across all Goose sessions
                            </span>
                          </label>
                          <label className="flex items-center">
                            <input
                              type="radio"
                              name="create-save-location"
                              checked={field.state.value === false}
                              onChange={() => field.handleChange(false)}
                              className="mr-2"
                            />
                            <span className="text-sm text-text-standard">
                              Directory - Available in the working directory
                            </span>
                          </label>
                        </div>
                      </div>
                    )}
                  </createRecipeForm.Field>
                    </div>
                  </motion.div>
                )}
              </AnimatePresence>
            </div>
          </div>

          <div className="flex justify-end space-x-3 mt-6">
            <Button type="button" onClick={handleClose} variant="ghost" disabled={creating}>
              Cancel
            </Button>
            <createRecipeForm.Subscribe
              selector={(state) => [state.canSubmit, state.isSubmitting, state.isValid]}
            >
              {([canSubmit, isSubmitting, isValid]) => {
                // Debug logging to see what's happening
                console.log('Form state:', { canSubmit, isSubmitting, isValid });

                return (
                  <Button
                    type="submit"
                    disabled={!canSubmit || creating || isSubmitting}
                    variant="default"
                  >
                    {creating || isSubmitting ? 'Creating...' : 'Create Recipe'}
                  </Button>
                );
              }}
            </createRecipeForm.Subscribe>
          </div>
        </form>
      </div>
    </div>
  );
}

// Export the button component for easy access
export function CreateRecipeButton({ onClick }: { onClick: () => void }) {
  return (
    <Button onClick={onClick} variant="outline" size="sm" className="flex items-center gap-2">
      <FileText className="w-4 h-4" />
      Create Recipe
    </Button>
  );
}
