import { useEffect, useMemo, useState } from 'react';
import { createRecipe, Recipe } from '../recipe';
import { Message, createUserMessage } from '../types/message';
import { updateSystemPromptWithParameters } from '../utils/providerUtils';
import { useChatContext } from '../contexts/ChatContext';

interface LocationState {
  recipeConfig?: Recipe;
  disableAnimation?: boolean;
  reset?: boolean;
}

export const useRecipeManager = (messages: Message[], locationState?: LocationState) => {
  const [isGeneratingRecipe, setIsGeneratingRecipe] = useState(false);
  const [isParameterModalOpen, setIsParameterModalOpen] = useState(false);
  const [recipeParameters, setRecipeParameters] = useState<Record<string, string> | null>(null);
  const [readyForAutoUserPrompt, setReadyForAutoUserPrompt] = useState(false);

  // Get chat context to access persisted recipe
  const { chat, setRecipeConfig } = useChatContext();

  // Get recipeConfig from multiple sources with priority:
  // 1. Chat context (persisted recipe)
  // 2. Navigation state (from recipes view)
  // 3. App config (from deeplinks)
  const recipeConfig = useMemo(() => {
    // First check if we have a persisted recipe in chat context
    if (chat.recipeConfig) {
      return chat.recipeConfig;
    }

    // Then check if recipe config is passed via navigation state
    if (locationState?.recipeConfig) {
      // Persist the recipe to chat context for future use
      setRecipeConfig(locationState.recipeConfig);
      return locationState.recipeConfig as Recipe;
    }

    // Fallback to app config (from deeplinks)
    const appRecipeConfig = window.appConfig.get('recipeConfig') as Recipe | null;
    if (appRecipeConfig) {
      // Persist the recipe to chat context for future use
      setRecipeConfig(appRecipeConfig);
      return appRecipeConfig;
    }

    return null;
  }, [chat.recipeConfig, locationState, setRecipeConfig]);

  // Show parameter modal if recipe has parameters and they haven't been set yet
  useEffect(() => {
    if (recipeConfig?.parameters && recipeConfig.parameters.length > 0) {
      // If we have parameters and they haven't been set yet, open the modal.
      if (!recipeParameters) {
        setIsParameterModalOpen(true);
      }
    }
  }, [recipeConfig, recipeParameters]);

  // Set ready for auto user prompt after component initialization
  useEffect(() => {
    setReadyForAutoUserPrompt(true);
  }, []);

  // Substitute parameters in prompt
  const substituteParameters = (prompt: string, params: Record<string, string>): string => {
    let substitutedPrompt = prompt;

    for (const key in params) {
      // Escape special characters in the key (parameter) and match optional whitespace
      const regex = new RegExp(`{{\\s*${key.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\s*}}`, 'g');
      substitutedPrompt = substitutedPrompt.replace(regex, params[key]);
    }
    return substitutedPrompt;
  };

  // Pre-fill input with recipe prompt instead of auto-sending it
  const initialPrompt = useMemo(() => {
    if (!recipeConfig?.prompt) return '';

    const hasRequiredParams = recipeConfig.parameters && recipeConfig.parameters.length > 0;

    // If params are required and have been collected, substitute them into the prompt.
    if (hasRequiredParams && recipeParameters) {
      return substituteParameters(recipeConfig.prompt, recipeParameters);
    }

    // If there are no parameters, return the original prompt.
    if (!hasRequiredParams) {
      return recipeConfig.prompt;
    }

    // Otherwise, we are waiting for parameters, so the input should be empty.
    return '';
  }, [recipeConfig, recipeParameters]);

  // Handle parameter submission
  const handleParameterSubmit = async (inputValues: Record<string, string>) => {
    setRecipeParameters(inputValues);
    setIsParameterModalOpen(false);

    // Update the system prompt with parameter-substituted instructions
    try {
      await updateSystemPromptWithParameters(inputValues);
    } catch (error) {
      console.error('Failed to update system prompt with parameters:', error);
    }
  };

  // Auto-execution handler for scheduled recipes
  const handleAutoExecution = (append: (message: Message) => void, isLoading: boolean) => {
    const hasRequiredParams = recipeConfig?.parameters && recipeConfig.parameters.length > 0;

    if (
      recipeConfig?.isScheduledExecution &&
      recipeConfig?.prompt &&
      (!hasRequiredParams || recipeParameters) &&
      messages.length === 0 &&
      !isLoading &&
      readyForAutoUserPrompt
    ) {
      // Substitute parameters if they exist
      const finalPrompt = recipeParameters
        ? substituteParameters(recipeConfig.prompt, recipeParameters)
        : recipeConfig.prompt;

      console.log('Auto-sending substituted prompt for scheduled execution:', finalPrompt);

      const userMessage = createUserMessage(finalPrompt);
      append(userMessage);
    }
  };

  // Listen for make-agent-from-chat event
  useEffect(() => {
    const handleMakeAgent = async () => {
      window.electron.logInfo('Making recipe from chat...');
      setIsGeneratingRecipe(true);

      try {
        // Create recipe directly from chat messages
        const createRecipeRequest = {
          messages: messages,
          title: '',
          description: '',
        };

        const response = await createRecipe(createRecipeRequest);

        if (response.error) {
          throw new Error(`Failed to create recipe: ${response.error}`);
        }

        window.electron.logInfo('Created recipe:');
        window.electron.logInfo(JSON.stringify(response.recipe, null, 2));

        // First, verify the recipe data
        if (!response.recipe) {
          throw new Error('No recipe data received');
        }

        // Create a new window for the recipe editor
        console.log('Opening recipe editor with config:', response.recipe);
        const recipeConfig = {
          id: response.recipe.title || 'untitled',
          title: response.recipe.title || 'Untitled Recipe',
          description: response.recipe.description || '',
          instructions: response.recipe.instructions || '',
          activities: response.recipe.activities || [],
          prompt: response.recipe.prompt || '',
        };
        window.electron.createChatWindow(
          undefined, // query
          undefined, // dir
          undefined, // version
          undefined, // resumeSessionId
          recipeConfig, // recipe config
          'recipeEditor' // view type
        );

        window.electron.logInfo('Opening recipe editor window');
      } catch (error) {
        window.electron.logInfo('Failed to create recipe:');
        const errorMessage = error instanceof Error ? error.message : String(error);
        window.electron.logInfo(errorMessage);
      } finally {
        setIsGeneratingRecipe(false);
      }
    };

    window.addEventListener('make-agent-from-chat', handleMakeAgent);

    return () => {
      window.removeEventListener('make-agent-from-chat', handleMakeAgent);
    };
  }, [messages]);

  return {
    recipeConfig,
    initialPrompt,
    isGeneratingRecipe,
    isParameterModalOpen,
    setIsParameterModalOpen,
    recipeParameters,
    handleParameterSubmit,
    handleAutoExecution,
  };
};
