import { useEffect, useMemo, useState, useRef } from 'react';
import { createRecipe, Recipe, scanRecipe } from '../recipe';
import { Message, createUserMessage } from '../types/message';
import {
  updateSystemPromptWithParameters,
  substituteParameters,
  filterValidUsedParameters,
} from '../utils/providerUtils';
import { useChatContext } from '../contexts/ChatContext';
import { ChatType } from '../types/chat';

export const useRecipeManager = (chat: ChatType, recipeConfig?: Recipe | null) => {
  const [isGeneratingRecipe, setIsGeneratingRecipe] = useState(false);
  const [isParameterModalOpen, setIsParameterModalOpen] = useState(false);
  const [readyForAutoUserPrompt, setReadyForAutoUserPrompt] = useState(false);
  const [recipeError, setRecipeError] = useState<string | null>(null);
  const [isRecipeWarningModalOpen, setIsRecipeWarningModalOpen] = useState(false);
  const [recipeAccepted, setRecipeAccepted] = useState(false);
  const [hasSecurityWarnings, setHasSecurityWarnings] = useState(false);

  const [recipeParameters, setRecipeParameters] = useState<Record<string, string> | null>(null);

  const chatContext = useChatContext();
  const messages = chat.messages;

  const messagesRef = useRef(messages);
  const isCreatingRecipeRef = useRef(false);

  useEffect(() => {
    messagesRef.current = messages;
  }, [messages]);

  const finalRecipeConfig = chat.recipeConfig;

  useEffect(() => {
    if (!chatContext) return;

    // If we have a recipe from navigation state, persist it
    if (recipeConfig && !chatContext.chat.recipeConfig) {
      chatContext.setRecipeConfig(recipeConfig);
      return;
    }

    // If we have a recipe from app config (deeplink), persist it
    // But only if the chat context doesn't explicitly have null (which indicates it was cleared)
    const appRecipeConfig = window.appConfig.get('recipe') as Recipe | null;
    if (appRecipeConfig && chatContext.chat.recipeConfig === undefined) {
      chatContext.setRecipeConfig(appRecipeConfig);
    }
  }, [chatContext, recipeConfig]);

  useEffect(() => {
    const checkRecipeAcceptance = async () => {
      if (finalRecipeConfig) {
        try {
          const hasAccepted = await window.electron.hasAcceptedRecipeBefore(finalRecipeConfig);

          if (!hasAccepted) {
            const securityScanResult = await scanRecipe(finalRecipeConfig);
            setHasSecurityWarnings(securityScanResult.has_security_warnings);

            setIsRecipeWarningModalOpen(true);
          } else {
            setRecipeAccepted(true);
          }
        } catch {
          setHasSecurityWarnings(false);
          setIsRecipeWarningModalOpen(true);
        }
      }
    };

    checkRecipeAcceptance();
  }, [finalRecipeConfig]);

  // Filter parameters to only show valid ones that are actually used in the recipe
  const filteredParameters = useMemo(() => {
    if (!finalRecipeConfig?.parameters) {
      return [];
    }
    return filterValidUsedParameters(finalRecipeConfig.parameters, {
      prompt: finalRecipeConfig.prompt || undefined,
      instructions: finalRecipeConfig.instructions || undefined,
    });
  }, [finalRecipeConfig]);

  // Check if template variables are actually used in the recipe content
  const requiresParameters = useMemo(() => {
    return filteredParameters.length > 0;
  }, [filteredParameters]);
  const hasParameters = !!recipeParameters;
  const hasMessages = messages.length > 0;
  useEffect(() => {
    if (requiresParameters && recipeAccepted) {
      if (!hasParameters && !hasMessages) {
        setIsParameterModalOpen(true);
      }
    }
  }, [requiresParameters, hasParameters, recipeAccepted, hasMessages]);

  useEffect(() => {
    setReadyForAutoUserPrompt(true);
  }, []);

  const initialPrompt = useMemo(() => {
    if (!finalRecipeConfig?.prompt || !recipeAccepted || finalRecipeConfig?.isScheduledExecution) {
      return '';
    }

    if (requiresParameters && recipeParameters) {
      return substituteParameters(finalRecipeConfig.prompt, recipeParameters);
    }

    return finalRecipeConfig.prompt;
  }, [finalRecipeConfig, recipeParameters, recipeAccepted, requiresParameters]);

  const handleParameterSubmit = async (inputValues: Record<string, string>) => {
    setRecipeParameters(inputValues);
    setIsParameterModalOpen(false);

    try {
      await updateSystemPromptWithParameters(
        chat.sessionId,
        inputValues,
        finalRecipeConfig || undefined
      );
    } catch (error) {
      console.error('Failed to update system prompt with parameters:', error);
    }
  };

  const handleRecipeAccept = async () => {
    try {
      if (finalRecipeConfig) {
        await window.electron.recordRecipeHash(finalRecipeConfig);
        setRecipeAccepted(true);
        setIsRecipeWarningModalOpen(false);
      }
    } catch (error) {
      console.error('Error recording recipe hash:', error);
      setRecipeAccepted(true);
      setIsRecipeWarningModalOpen(false);
    }
  };

  const handleRecipeCancel = () => {
    setIsRecipeWarningModalOpen(false);
    window.electron.closeWindow();
  };

  const handleAutoExecution = (
    append: (message: Message) => void,
    isLoading: boolean,
    onAutoExecute?: () => void
  ) => {
    if (
      finalRecipeConfig?.isScheduledExecution &&
      finalRecipeConfig?.prompt &&
      (!requiresParameters || recipeParameters) &&
      messages.length === 0 &&
      !isLoading &&
      readyForAutoUserPrompt &&
      recipeAccepted
    ) {
      const finalPrompt = recipeParameters
        ? substituteParameters(finalRecipeConfig.prompt, recipeParameters)
        : finalRecipeConfig.prompt;

      console.log('Auto-sending substituted prompt for scheduled execution:', finalPrompt);

      const userMessage = createUserMessage(finalPrompt);
      append(userMessage);
      onAutoExecute?.();
    }
  };

  useEffect(() => {
    const handleMakeAgent = async () => {
      if (window.isCreatingRecipe) {
        return;
      }

      if (isCreatingRecipeRef.current) {
        return;
      }

      window.electron.logInfo('Making recipe from chat...');

      isCreatingRecipeRef.current = true;
      window.isCreatingRecipe = true;
      setIsGeneratingRecipe(true);

      try {
        const createRecipeRequest = {
          messages: messagesRef.current,
          title: '',
          description: '',
          session_id: chat.sessionId,
        };

        const response = await createRecipe(createRecipeRequest);

        if (response.error) {
          throw new Error(`Failed to create recipe: ${response.error}`);
        }

        window.electron.logInfo('Created recipe successfully');

        if (!response.recipe) {
          throw new Error('No recipe data received');
        }

        window.sessionStorage.setItem('ignoreRecipeConfigChanges', 'true');

        window.electron.createChatWindow(
          undefined,
          undefined,
          undefined,
          undefined,
          response.recipe,
          'recipeEditor'
        );

        window.electron.logInfo('Opening recipe editor window');

        setTimeout(() => {
          window.sessionStorage.removeItem('ignoreRecipeConfigChanges');
        }, 1000);
      } catch (error) {
        window.electron.logInfo('Failed to create recipe:');
        const errorMessage = error instanceof Error ? error.message : String(error);
        window.electron.logInfo(errorMessage);

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
  }, [chat.sessionId]);

  return {
    recipeConfig: finalRecipeConfig,
    filteredParameters,
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
    hasSecurityWarnings,
  };
};
