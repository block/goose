import { useEffect, useRef, useState } from 'react';
import { useForm } from '@tanstack/react-form';
import { Recipe } from '../../recipe';
import { Geese } from '../icons/Geese';
import { X, Save, Play, Loader2 } from 'lucide-react';
import { Button } from '../ui/button';
import { RecipeFormFields } from './shared/RecipeFormFields';
import { RecipeFormData } from './shared/recipeFormSchema';
import { createRecipe } from '../../api/sdk.gen';
import { RecipeParameter } from './shared/recipeFormSchema';
import { toastError } from '../../toasts';
import { saveRecipe } from '../../recipe/recipe_management';
import { errorMessage } from '../../utils/conversionUtils';
import { useLocalization } from '../../contexts/LocalizationContext';

interface CreateRecipeFromSessionModalProps {
  isOpen: boolean;
  onClose: () => void;
  sessionId: string;
  onRecipeCreated?: (recipe: Recipe) => void;
}

export default function CreateRecipeFromSessionModal({
  isOpen,
  onClose,
  sessionId,
  onRecipeCreated,
}: CreateRecipeFromSessionModalProps) {
  const { t } = useLocalization();
  const [isCreating, setIsCreating] = useState(false);
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [analysisStage, setAnalysisStage] = useState<string>('');
  const [hasAnalyzed, setHasAnalyzed] = useState(false);
  const tRef = useRef(t);
  const hasAnalyzedRef = useRef(false);

  // Initialize form with empty values for new recipe
  const form = useForm({
    defaultValues: {
      title: '',
      description: '',
      instructions: '',
      prompt: '',
      activities: [] as string[],
      parameters: [] as RecipeParameter[],
      jsonSchema: '',
      recipeName: '',
      global: true,
    } as RecipeFormData,
    onSubmit: async ({ value }) => {
      await handleCreateRecipe(value);
    },
  });

  // Track form validity with state to make it reactive
  const [isFormValid, setIsFormValid] = useState(false);

  useEffect(() => {
    tRef.current = t;
  }, [t]);

  useEffect(() => {
    hasAnalyzedRef.current = hasAnalyzed;
  }, [hasAnalyzed]);

  // Analyze messages and prefill form when modal opens
  useEffect(() => {
    if (!isOpen || !sessionId || hasAnalyzedRef.current) {
      return;
    }

    let cancelled = false;
    let stageTimeout: ReturnType<typeof setTimeout> | null = null;

    setIsAnalyzing(true);

    // Create a sequence of analysis stages for better UX
    const stages = [
      tRef.current('recipes.createFromSession.stages.readingConversation'),
      tRef.current('recipes.createFromSession.stages.identifyingPatterns'),
      tRef.current('recipes.createFromSession.stages.extractingTopics'),
      tRef.current('recipes.createFromSession.stages.generatingStructure'),
      tRef.current('recipes.createFromSession.stages.finalizing'),
    ];

    let currentStageIndex = 0;
    setAnalysisStage(stages[0]);

    // Update stage every 800ms
    const stageInterval = setInterval(() => {
      currentStageIndex = (currentStageIndex + 1) % stages.length;
      setAnalysisStage(stages[currentStageIndex]);
    }, 800);

    // Call the backend to analyze messages and create a recipe
    createRecipe({
      body: { session_id: sessionId },
      throwOnError: true,
    })
      .then((response) => {
        if (cancelled) {
          return;
        }

        clearInterval(stageInterval);
        setAnalysisStage(tRef.current('recipes.createFromSession.complete'));

        if (response.data?.recipe) {
          const recipe = response.data.recipe;

          // Prefill the form with the analyzed recipe information
          form.setFieldValue('title', recipe.title || '');
          form.setFieldValue('description', recipe.description || '');
          form.setFieldValue('instructions', recipe.instructions || '');
          form.setFieldValue('activities', recipe.activities || []);
          form.setFieldValue('parameters', recipe.parameters || []);

          if (recipe.response?.json_schema) {
            form.setFieldValue(
              'jsonSchema',
              JSON.stringify(recipe.response.json_schema, null, 2)
            );
          }
        } else {
          console.error('No recipe in response:', response);
        }
        setHasAnalyzed(true);
      })
      .catch((error) => {
        if (cancelled) {
          return;
        }

        console.error('Failed to analyze messages:', error);
        setAnalysisStage(tRef.current('recipes.createFromSession.analysisFailed'));
      })
      .finally(() => {
        clearInterval(stageInterval);
        if (cancelled) {
          return;
        }

        setHasAnalyzed(true);
        stageTimeout = setTimeout(() => {
          if (cancelled) {
            return;
          }
          setIsAnalyzing(false);
          setAnalysisStage('');
        }, 500); // Brief delay to show completion
      });

    return () => {
      cancelled = true;
      clearInterval(stageInterval);
      if (stageTimeout) {
        clearTimeout(stageTimeout);
      }
    };
  }, [isOpen, sessionId, form]);

  // Reset analysis state when modal closes
  useEffect(() => {
    if (!isOpen) {
      hasAnalyzedRef.current = false;
      setHasAnalyzed(false);
      setIsAnalyzing(false);
      setAnalysisStage('');
    }
  }, [isOpen]);

  // Subscribe to form changes using the form's subscribe method
  useEffect(() => {
    const unsubscribe = form.store.subscribe(() => {
      const hasTitle = form.state.values.title?.trim();
      const hasDescription = form.state.values.description?.trim();
      const hasInstructions = form.state.values.instructions?.trim();
      const valid = !!(hasTitle && hasDescription && hasInstructions);

      setIsFormValid(valid);
    });

    // Initial validation check
    const hasTitle = form.state.values.title?.trim();
    const hasDescription = form.state.values.description?.trim();
    const hasInstructions = form.state.values.instructions?.trim();
    const valid = !!(hasTitle && hasDescription && hasInstructions);
    setIsFormValid(valid);

    return unsubscribe;
  }, [form]);

  const handleCreateRecipe = async (formData: RecipeFormData, runAfterSave = false) => {
    if (!isFormValid) {
      return;
    }

    setIsCreating(true);
    try {
      const recipe: Recipe = {
        title: formData.title,
        description: formData.description,
        instructions: formData.instructions,
        prompt: formData.prompt || undefined,
        activities: formData.activities.filter((activity) => activity.trim() !== ''),
        parameters: formData.parameters.map((param) => ({
          key: param.key,
          input_type: param.input_type || 'string',
          requirement: param.requirement,
          description: param.description,
          ...(param.requirement === 'optional' && param.default ? { default: param.default } : {}),
          ...(param.input_type === 'select' && param.options
            ? {
                options: param.options.filter((opt: string) => opt.trim() !== ''),
              }
            : {}),
        })),
        response:
          formData.jsonSchema && formData.jsonSchema.trim()
            ? {
                json_schema: JSON.parse(formData.jsonSchema),
              }
            : undefined,
      };

      const recipeId = await saveRecipe(recipe, null);

      onRecipeCreated?.(recipe);
      onClose();

      if (runAfterSave) {
        window.electron.createChatWindow({ recipeId });
      }
    } catch (error) {
      console.error('Failed to create recipe:', error);
      toastError({
        title: t('recipes.createFromSession.createFailedTitle'),
        msg: errorMessage(error, t('recipes.createFromSession.createFailedMessage')),
      });
    } finally {
      setIsCreating(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-[400] flex items-center justify-center bg-black/50 p-4"
      data-testid="create-recipe-modal"
    >
      <div className="bg-background-primary border border-border-primary rounded-lg w-full max-w-4xl h-full max-h-[90vh] flex flex-col shadow-xl">
        {/* Header */}
        <div
          className="flex items-center justify-between p-6 border-b border-border-primary shrink-0"
          data-testid="modal-header"
        >
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 bg-background-primary rounded-full flex items-center justify-center">
              <Geese className="w-6 h-6 text-iconProminent" />
            </div>
            <div>
              <h1 className="text-xl font-medium text-text-primary">
                {t('recipes.createFromSession.title')}
              </h1>
              <p className="text-text-secondary text-sm">
                {t('recipes.createFromSession.description')}
              </p>
            </div>
          </div>
          <Button
            onClick={onClose}
            variant="ghost"
            size="sm"
            className="p-2 hover:bg-background-secondary rounded-lg transition-colors"
            data-testid="close-button"
          >
            <X className="w-5 h-5" />
          </Button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto px-6 py-4 min-h-0" data-testid="modal-content">
          {isAnalyzing ? (
            <div
              className="flex flex-col items-center justify-center h-full min-h-[300px] space-y-4"
              data-testid="analyzing-state"
            >
              <div className="flex items-center space-x-3">
                <Loader2
                  className="w-6 h-6 animate-spin text-iconProminent"
                  data-testid="analysis-spinner"
                />
                <div
                  className="text-lg font-medium text-text-primary"
                  data-testid="analyzing-title"
                >
                  {t('recipes.createFromSession.analyzingTitle')}
                </div>
              </div>
              <div
                className="text-text-secondary text-center max-w-md"
                data-testid="analysis-stage"
              >
                {analysisStage}
              </div>
              <div className="flex items-center space-x-2 text-text-secondary">
                <Geese className="w-5 h-5 animate-pulse" />
                <span className="text-sm">{t('recipes.createFromSession.extractingInsights')}</span>
              </div>
            </div>
          ) : (
            <div data-testid="form-state">
              <RecipeFormFields form={form} />
            </div>
          )}
        </div>

        {/* Footer */}
        <div
          className="flex items-center justify-between p-6 border-t border-border-primary shrink-0"
          data-testid="modal-footer"
        >
          <Button
            onClick={onClose}
            variant="ghost"
            className="px-4 py-2 text-text-secondary rounded-lg hover:bg-background-secondary transition-colors"
            data-testid="cancel-button"
          >
            {t('common.actions.cancel')}
          </Button>

          <div className="flex gap-3">
            {!isAnalyzing && (
              <>
                <Button
                  onClick={() => {
                    form.handleSubmit();
                  }}
                  disabled={!isFormValid || isCreating}
                  variant="outline"
                  className="px-4 py-2 border border-border-primary rounded-lg hover:bg-background-secondary transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                  data-testid="create-recipe-button"
                >
                  <Save className="w-4 h-4 mr-2" />
                  {isCreating
                    ? t('recipes.createFromSession.creating')
                    : t('recipes.createFromSession.create')}
                </Button>
                <Button
                  onClick={() => {
                    handleCreateRecipe(form.state.values, true);
                  }}
                  disabled={!isFormValid || isCreating}
                  className="px-4 py-2 text-text-inverse rounded-lg hover:bg-opacity-90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                  data-testid="create-and-run-recipe-button"
                >
                  <Play className="w-4 h-4 mr-2" />
                  {isCreating
                    ? t('recipes.createFromSession.creating')
                    : t('recipes.createFromSession.createAndRun')}
                </Button>
              </>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
