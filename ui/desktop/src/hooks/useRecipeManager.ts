import { useEffect, useMemo, useState, useRef } from 'react';
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
  const [readyForAutoUserPrompt, setReadyForAutoUserPrompt] = useState(false);
  const [recipeError, setRecipeError] = useState<string | null>(null);
  const [isRecipeWarningModalOpen, setIsRecipeWarningModalOpen] = useState(false);
  const [recipeAccepted, setRecipeAccepted] = useState(false);

  // Get chat context to access persisted recipe and parameters
  const chatContext = useChatContext();

  // Use a ref to capture the current messages for the event handler
  const messagesRef = useRef(messages);
  const isCreatingRecipeRef = useRef(false);

  // Update the ref when messages change
  useEffect(() => {
    messagesRef.current = messages;
  }, [messages]);

  // Get recipeConfig from multiple sources with priority:
  // 1. Chat context (persisted recipe)
  // 2. Navigation state (from recipes view)
  // 3. App config (from deeplinks)
  const recipeConfig = useMemo(() => {
    // First check if we have a persisted recipe in chat context
    // We need to distinguish between null (explicitly cleared) and undefined (not set)
    if (chatContext?.chat.recipeConfig !== undefined) {
      return chatContext.chat.recipeConfig; // This could be null or a Recipe
    }

    // Then check if recipe config is passed via navigation state
    if (locationState?.recipeConfig) {
      return locationState.recipeConfig as Recipe;
    }

    // Fallback to app config (from deeplinks)
    const appRecipeConfig = window.appConfig.get('recipe') as Recipe | null;
    if (appRecipeConfig) {
      return appRecipeConfig;
    }
    return null;
  }, [chatContext, locationState]);

  // Get recipe parameters from chat context
  const recipeParameters = useMemo(() => {
    return chatContext?.chat.recipeParameters || null;
  }, [chatContext?.chat.recipeParameters]);

  // Effect to persist recipe config to chat context when it changes
  useEffect(() => {
    if (!chatContext?.setRecipeConfig) return;

    // If we have a recipe from navigation state, persist it
    if (locationState?.recipeConfig && !chatContext.chat.recipeConfig) {
      chatContext.setRecipeConfig(locationState.recipeConfig);
      return;
    }

    // If we have a recipe from app config (deeplink), persist it
    // But only if the chat context doesn't explicitly have null (which indicates it was cleared)
    const appRecipeConfig = window.appConfig.get('recipe') as Recipe | null;
    if (appRecipeConfig && chatContext.chat.recipeConfig === undefined) {
      // Only set if recipeConfig is undefined, not if it's explicitly null
      chatContext.setRecipeConfig(appRecipeConfig);
    }
  }, [chatContext, locationState]);

  // Check if recipe has been accepted before
  useEffect(() => {
    const checkRecipeAcceptance = async () => {
      if (recipeConfig) {
        try {
          const hasAccepted = await window.electron.hasAcceptedRecipeBefore(recipeConfig);
          if (!hasAccepted) {
            setIsRecipeWarningModalOpen(true);
          } else {
            setRecipeAccepted(true);
          }
        } catch (error) {
          console.error('Error checking recipe acceptance:', error);
          // If there's an error, assume the recipe hasn't been accepted
          setIsRecipeWarningModalOpen(true);
        }
      }
    };

    checkRecipeAcceptance();
  }, [recipeConfig]);

  // Show parameter modal if recipe has parameters and they haven't been set yet
  useEffect(() => {
    if (recipeConfig?.parameters && recipeConfig.parameters.length > 0 && recipeAccepted) {
      // If we have parameters and they haven't been set yet, open the modal.
      if (!recipeParameters) {
        setIsParameterModalOpen(true);
      }
    }
  }, [recipeConfig, recipeParameters, recipeAccepted]);

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

  // Get the recipe's initial prompt (always return the actual prompt, don't modify based on conversation state)
  const initialPrompt = useMemo(() => {
    if (!recipeConfig?.prompt || !recipeAccepted || recipeConfig?.isScheduledExecution) {
      return '';
    }

    const hasRequiredParams = recipeConfig.parameters && recipeConfig.parameters.length > 0;

    // If params are required and have been collected, substitute them into the prompt.
    if (hasRequiredParams && recipeParameters) {
      return substituteParameters(recipeConfig.prompt, recipeParameters);
    }

    // Always return the original prompt, whether it has parameters or not
    // The user should see the prompt with parameter placeholders before filling them in
    return recipeConfig.prompt;
  }, [recipeConfig, recipeParameters, recipeAccepted]);

  // Handle parameter submission
  const handleParameterSubmit = async (inputValues: Record<string, string>) => {
    // Store parameters in chat context instead of local state
    if (chatContext?.setRecipeParameters) {
      chatContext.setRecipeParameters(inputValues);
    }
    setIsParameterModalOpen(false);

    // Update the system prompt with parameter-substituted instructions
    try {
      await updateSystemPromptWithParameters(inputValues, recipeConfig || undefined);
    } catch (error) {
      console.error('Failed to update system prompt with parameters:', error);
    }
  };

  // Handle recipe acceptance
  const handleRecipeAccept = async () => {
    try {
      if (recipeConfig) {
        await window.electron.recordRecipeHash(recipeConfig);
        setRecipeAccepted(true);
        setIsRecipeWarningModalOpen(false);
      }
    } catch (error) {
      console.error('Error recording recipe hash:', error);
      // Even if recording fails, we should still allow the user to proceed
      setRecipeAccepted(true);
      setIsRecipeWarningModalOpen(false);
    }
  };

  // Handle recipe cancellation
  const handleRecipeCancel = () => {
    setIsRecipeWarningModalOpen(false);
    window.electron.closeWindow();
  };

  // Auto-execution handler for scheduled recipes
  const handleAutoExecution = (
    append: (message: Message) => void,
    isLoading: boolean,
    onAutoExecute?: () => void
  ) => {
    const hasRequiredParams = recipeConfig?.parameters && recipeConfig.parameters.length > 0;

    if (
      recipeConfig?.isScheduledExecution &&
      recipeConfig?.prompt &&
      (!hasRequiredParams || recipeParameters) &&
      messages.length === 0 &&
      !isLoading &&
      readyForAutoUserPrompt &&
      recipeAccepted
    ) {
      // Substitute parameters if they exist
      const finalPrompt = recipeParameters
        ? substituteParameters(recipeConfig.prompt, recipeParameters)
        : recipeConfig.prompt;

      console.log('Auto-sending substituted prompt for scheduled execution:', finalPrompt);

      const userMessage = createUserMessage(finalPrompt);
      append(userMessage);
      onAutoExecute?.();
    }
  };

  // Listen for make-agent-from-chat event
  useEffect(() => {
    const handleMakeAgent = async () => {
      // Prevent duplicate calls using global flag
      if (window.isCreatingRecipe) {
        return;
      }

      // Prevent duplicate calls using local ref
      if (isCreatingRecipeRef.current) {
        return;
      }

      window.electron.logInfo('Making recipe from chat...');

      // Set both local and global flags
      isCreatingRecipeRef.current = true;
      window.isCreatingRecipe = true;
      setIsGeneratingRecipe(true);

      try {
        // Create recipe directly from chat messages using the ref to get current messages
        const createRecipeRequest = {
          messages: messagesRef.current,
          title: '',
          description: '',
        };

        const response = await createRecipe(createRecipeRequest);

        if (response.error) {
          throw new Error(`Failed to create recipe: ${response.error}`);
        }

        window.electron.logInfo('Created recipe successfully');

        // Verify the recipe data
        if (!response.recipe) {
          throw new Error('No recipe data received');
        }

        // Set a flag to prevent the current window from reacting to recipe config changes
        // This prevents navigation conflicts when creating new windows
        window.sessionStorage.setItem('ignoreRecipeConfigChanges', 'true');

        // Create the new window
        window.electron.createChatWindow(
          undefined, // query
          undefined, // dir
          undefined, // version
          undefined, // resumeSessionId
          response.recipe, // recipe config
          'recipeEditor' // view type
        );

        window.electron.logInfo('Opening recipe editor window');

        // Clear the ignore flag after a short delay
        setTimeout(() => {
          window.sessionStorage.removeItem('ignoreRecipeConfigChanges');
        }, 1000);
      } catch (error) {
        window.electron.logInfo('Failed to create recipe:');
        const errorMessage = error instanceof Error ? error.message : String(error);
        window.electron.logInfo(errorMessage);

        // Set the error state to show in modal
        setRecipeError(errorMessage);
      } finally {
        isCreatingRecipeRef.current = false;
        window.isCreatingRecipe = false;
        setIsGeneratingRecipe(false);
      }
    };

    window.addEventListener('make-agent-from-chat', handleMakeAgent);

    return () => {
      window.removeEventListener('make-agent-from-chat', handleMakeAgent);
    };
  }, []);

  return {
    recipeConfig,
    initialPrompt,
    isGeneratingRecipe,
    isParameterModalOpen,
    setIsParameterModalOpen,
    recipeParameters,
    handleParameterSubmit,
    handleAutoExecution,
    recipeError,
    setRecipeError,
    isRecipeWarningModalOpen,
    setIsRecipeWarningModalOpen,
    recipeAccepted,
    handleRecipeAccept,
    handleRecipeCancel,
  };
};
