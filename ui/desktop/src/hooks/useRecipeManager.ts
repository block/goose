import { useEffect, useMemo, useState, useRef } from 'react';
import { Recipe, scanRecipe } from '../recipe';
import { createUserMessage } from '../types/message';
import { Message } from '../api';

import { updateSystemPromptWithParameters, substituteParameters } from '../utils/providerUtils';
import { updateSessionUserRecipeValues } from '../api';
import { useChatContext } from '../contexts/ChatContext';
import { ChatType } from '../types/chat';
import { toastSuccess } from '../toasts';

export const useRecipeManager = (chat: ChatType, recipe?: Recipe | null) => {
  const [isParameterModalOpen, setIsParameterModalOpen] = useState(false);
  const [isRecipeWarningModalOpen, setIsRecipeWarningModalOpen] = useState(false);
  const [recipeAccepted, setRecipeAccepted] = useState(false);
  const [isCreateRecipeModalOpen, setIsCreateRecipeModalOpen] = useState(false);
  const [hasSecurityWarnings, setHasSecurityWarnings] = useState(false);
  const [readyForAutoUserPrompt, setReadyForAutoUserPrompt] = useState(false);
  const [recipeError, setRecipeError] = useState<string | null>(null);
  const recipeParameterValues = chat.recipeParameterValues;

  const chatContext = useChatContext();
  const messages = chat.messages;

  const messagesRef = useRef(messages);
  const isCreatingRecipeRef = useRef(false);
  const hasCheckedRecipeRef = useRef(false);

  useEffect(() => {
    messagesRef.current = messages;
  }, [messages]);

  const finalRecipe = chat.recipe;

  useEffect(() => {
    if (!chatContext) return;

    // If we have a recipe from navigation state, always set it and reset acceptance state
    // This ensures that when loading a new recipe, we start fresh
    if (recipe) {
      // Check if this is actually a different recipe (by comparing title and content)
      const currentRecipe = chatContext.chat.recipe;
      const isNewRecipe =
        !currentRecipe ||
        currentRecipe.title !== recipe.title ||
        currentRecipe.instructions !== recipe.instructions ||
        currentRecipe.prompt !== recipe.prompt ||
        JSON.stringify(currentRecipe.activities) !== JSON.stringify(recipe.activities);

      if (isNewRecipe) {
        console.log('Setting new recipe config:', recipe.title);
        // Reset recipe acceptance state when loading a new recipe
        setRecipeAccepted(false);
        setIsParameterModalOpen(false);
        setIsRecipeWarningModalOpen(false);
        hasCheckedRecipeRef.current = false; // Reset check flag for new recipe

        chatContext.setChat({
          ...chatContext.chat,
          recipe: recipe,
          recipeParameterValues: null,
          messages: [],
        });
      }
      return;
    }
  }, [chatContext, recipe]);

  useEffect(() => {
    const checkRecipeAcceptance = async () => {
      // Only check once per recipe load
      if (hasCheckedRecipeRef.current) {
        return;
      }

      if (finalRecipe) {
        hasCheckedRecipeRef.current = true;

        try {
          const hasAccepted = await window.electron.hasAcceptedRecipeBefore(finalRecipe);

          if (!hasAccepted) {
            const securityScanResult = await scanRecipe(finalRecipe);
            setHasSecurityWarnings(securityScanResult.has_security_warnings);

            setIsRecipeWarningModalOpen(true);
          } else {
            setRecipeAccepted(true);
          }
        } catch {
          setHasSecurityWarnings(false);
          setIsRecipeWarningModalOpen(true);
        }
      } else {
        setRecipeAccepted(false);
        setIsRecipeWarningModalOpen(false);
      }
    };

    checkRecipeAcceptance();
  }, [finalRecipe, recipe, chat.messages.length]);

  const filteredParameters = useMemo(() => {
    return finalRecipe?.parameters ?? [];
  }, [finalRecipe]);

  // Check if template variables are actually used in the recipe content
  const requiresParameters = useMemo(() => {
    return filteredParameters.length > 0;
  }, [filteredParameters]);

  // Check if all required parameters have been filled in
  const hasAllRequiredParameters = useMemo(() => {
    if (!requiresParameters) {
      return true; // No parameters required, so all are "filled"
    }

    if (!recipeParameterValues) {
      return false; // Parameters required but none provided
    }

    // Check if all filtered parameters have values
    return filteredParameters.every((param) => {
      const value = recipeParameterValues[param.key];
      return value !== undefined && value !== null && value.trim() !== '';
    });
  }, [filteredParameters, recipeParameterValues, requiresParameters]);

  const hasMessages = messages.length > 0;
  useEffect(() => {
    // Only show parameter modal if:
    // 1. Recipe requires parameters
    // 2. Recipe has been accepted
    // 3. Not all required parameters have been filled in yet
    // 4. Parameter modal is not already open (prevent multiple opens)
    // 5. No messages in chat yet (don't show after conversation has started)
    if (recipeAccepted && !hasAllRequiredParameters && !isParameterModalOpen && !hasMessages) {
      setIsParameterModalOpen(true);
    }
  }, [
    hasAllRequiredParameters,
    recipeAccepted,
    filteredParameters,
    isParameterModalOpen,
    hasMessages,
    chat.sessionId,
    finalRecipe?.title,
  ]);

  useEffect(() => {
    setReadyForAutoUserPrompt(true);
  }, []);

  const initialPrompt = useMemo(() => {
    if (!finalRecipe?.prompt || !recipeAccepted || finalRecipe?.isScheduledExecution) {
      return '';
    }

    if (requiresParameters && recipeParameterValues) {
      return substituteParameters(finalRecipe.prompt, recipeParameterValues);
    }

    return finalRecipe.prompt;
  }, [finalRecipe, recipeParameterValues, recipeAccepted, requiresParameters]);

  const handleParameterSubmit = async (inputValues: Record<string, string>) => {
    // Update chat state with parameters
    if (chatContext) {
      chatContext.setChat({
        ...chatContext.chat,
        recipeParameterValues: inputValues,
      });
    }
    setIsParameterModalOpen(false);

    try {
      await updateSystemPromptWithParameters(chat.sessionId, inputValues, finalRecipe || undefined);

      // Save recipe parameters to session metadata
      await updateSessionUserRecipeValues({
        path: {
          session_id: chat.sessionId,
        },
        body: {
          userRecipeValues: inputValues,
        },
        throwOnError: true,
      });
    } catch (error) {
      console.error('Failed to update system prompt with parameters:', error);
    }
  };

  const handleRecipeAccept = async () => {
    try {
      if (finalRecipe) {
        await window.electron.recordRecipeHash(finalRecipe);
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
      finalRecipe?.isScheduledExecution &&
      finalRecipe?.prompt &&
      (!requiresParameters || recipeParameterValues) &&
      messages.length === 0 &&
      !isLoading &&
      readyForAutoUserPrompt &&
      recipeAccepted
    ) {
      const finalPrompt = recipeParameterValues
        ? substituteParameters(finalRecipe.prompt, recipeParameterValues)
        : finalRecipe.prompt;

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

      setIsCreateRecipeModalOpen(true);
    };

    window.addEventListener('make-agent-from-chat', handleMakeAgent);

    return () => {
      window.removeEventListener('make-agent-from-chat', handleMakeAgent);
    };
  }, [chat.sessionId]);

  const handleRecipeCreated = (recipe: Recipe) => {
    toastSuccess({
      title: 'Recipe created successfully!',
      msg: `"${recipe.title}" has been saved and is ready to use.`,
    });
  };

  const recipeId: string | null =
    (window.appConfig.get('recipeId') as string | null | undefined) ?? null;

  return {
    recipe: finalRecipe,
    recipeId,
    recipeParameterValues,
    filteredParameters,
    initialPrompt,
    isParameterModalOpen,
    setIsParameterModalOpen,
    readyForAutoUserPrompt,
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
    isCreateRecipeModalOpen,
    setIsCreateRecipeModalOpen,
    handleRecipeCreated,
  };
};
