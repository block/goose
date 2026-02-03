import { useState, useEffect, useCallback, useMemo } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useForm } from '@tanstack/react-form';
import { ArrowLeft, Check, Play, Save, Sparkles } from 'lucide-react';

import { Recipe, Parameter, encodeRecipe, generateDeepLink } from '../../recipe';
import { Message } from '../../api';
import { UserInput } from '../../types/message';

// Type for parsed suggestions
interface Suggestion {
  field: string;
  value: string;
  fieldLabel: string;
}
import { Button } from '../ui/button';
import { RecipeFormFields } from './shared/RecipeFormFields';
import { RecipeFormData } from './shared/recipeFormSchema';
import { toastSuccess, toastError } from '../../toasts';
import { saveRecipe } from '../../recipe/recipe_management';
import { errorMessage } from '../../utils/conversionUtils';
import { createSession } from '../../sessions';
import { getInitialWorkingDir } from '../../utils/workingDir';
import { useChatStream } from '../../hooks/useChatStream';
import { ChatState } from '../../types/chatState';
import { useNavigation } from '../../hooks/useNavigation';
import { Geese } from '../icons/Geese';
import Copy from '../icons/Copy';
import ProgressiveMessageList from '../ProgressiveMessageList';
import ChatInput from '../ChatInput';
import LoadingGoose from '../LoadingGoose';
import { AppEvents } from '../../constants/events';

// System prompt for the Recipe Builder AI assistant
const RECIPE_BUILDER_SYSTEM_PROMPT = `You are a Recipe Builder assistant that helps users create Goose recipes through conversation.

A Goose recipe is a configuration that defines how an AI coding assistant should behave for a specific task.

Your job is to:
1. Ask the user what kind of recipe they want to create
2. Help them define the recipe fields through conversation
3. Provide suggestions they can apply to the form

Recipe fields:
- title: Short name for the recipe (3-100 characters)
- description: Brief explanation of what the recipe does (10-500 characters)
- instructions: Detailed step-by-step guidance for Goose
- prompt: Optional initial prompt to start conversations
- activities: Comma-separated list of activity types for the recipe
- parameters: JSON array of parameter objects with keys: key, description, input_type (string/select), requirement (required/optional)

When you have a suggestion for a specific field, format it as:
SUGGESTION[field_name]: <your suggested value>

For example:
SUGGESTION[title]: Code Review Assistant
SUGGESTION[description]: Reviews code for best practices, security issues, and performance
SUGGESTION[instructions]: You are a code review assistant. When the user shares code:
1. Analyze it for bugs and issues
2. Check for security vulnerabilities
3. Suggest performance improvements
4. Recommend best practices
SUGGESTION[prompt]: Please review my code
SUGGESTION[activities]: code-review, testing
SUGGESTION[parameters]: [{"key": "language", "description": "Programming language", "input_type": "string", "requirement": "optional"}]

The user can see a form on the right side that shows the current recipe state. Your suggestions will appear as "Apply" buttons they can click.

Start by asking what kind of recipe the user wants to create.`;

// Field labels for display
const FIELD_LABELS: Record<string, string> = {
  title: 'Title',
  description: 'Description',
  instructions: 'Instructions',
  prompt: 'Prompt',
  activities: 'Activities',
  parameters: 'Parameters',
};

// Parse suggestions from message content
function parseSuggestionsFromContent(content: string): Suggestion[] {
  const suggestions: Suggestion[] = [];
  // Match SUGGESTION[field]: value (where value can be multiline until next SUGGESTION or end)
  const regex = /SUGGESTION\[(\w+)\]:\s*([\s\S]*?)(?=SUGGESTION\[|$)/g;

  let match;
  while ((match = regex.exec(content)) !== null) {
    const field = match[1].toLowerCase();
    const value = match[2].trim();

    if (FIELD_LABELS[field] && value) {
      suggestions.push({
        field,
        value,
        fieldLabel: FIELD_LABELS[field],
      });
    }
  }

  return suggestions;
}

// Extract text content from a message
function getMessageTextContent(message: Message): string {
  return message.content
    .filter((c) => c.type === 'text')
    .map((c) => (c as { type: 'text'; text: string }).text)
    .join('\n');
}

export default function RecipeBuilderView() {
  const setView = useNavigation();
  const [searchParams, setSearchParams] = useSearchParams();

  // Session state
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [isCreatingSession, setIsCreatingSession] = useState(false);
  const [sessionError, setSessionError] = useState<string | null>(null);

  // Form state
  const getInitialValues = useCallback((): RecipeFormData => ({
    title: '',
    description: '',
    instructions: '',
    prompt: '',
    activities: [],
    parameters: [],
    jsonSchema: '',
  }), []);

  const form = useForm({
    defaultValues: getInitialValues(),
  });

  // Local state synced with form
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [instructions, setInstructions] = useState('');
  const [prompt, setPrompt] = useState('');
  const [activities, setActivities] = useState<string[]>([]);
  const [parameters, setParameters] = useState<Parameter[]>([]);
  const [jsonSchema, setJsonSchema] = useState('');
  const [copied, setCopied] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [deeplink, setDeeplink] = useState('');
  const [isGeneratingDeeplink, setIsGeneratingDeeplink] = useState(false);

  // Subscribe to form changes
  useEffect(() => {
    return form.store.subscribe(() => {
      setTitle(form.state.values.title);
      setDescription(form.state.values.description);
      setInstructions(form.state.values.instructions);
      setPrompt(form.state.values.prompt || '');
      setActivities(form.state.values.activities);
      setParameters(form.state.values.parameters);
      setJsonSchema(form.state.values.jsonSchema || '');
    });
  }, [form]);

  // Create session with recipe builder recipe on mount
  useEffect(() => {
    const existingSessionId = searchParams.get('sessionId');
    if (existingSessionId) {
      setSessionId(existingSessionId);
      return;
    }

    if (isCreatingSession || sessionId) return;

    const createBuilderSession = async () => {
      setIsCreatingSession(true);
      try {
        // Create a recipe with the builder system prompt
        const builderRecipe: Recipe = {
          title: 'Recipe Builder',
          description: 'AI assistant for creating Goose recipes',
          instructions: RECIPE_BUILDER_SYSTEM_PROMPT,
        };

        const recipeDeeplink = await encodeRecipe(builderRecipe);
        const workingDir = await getInitialWorkingDir();
        const session = await createSession(workingDir, { recipeDeeplink });

        setSessionId(session.id);
        setSearchParams((prev) => {
          prev.set('sessionId', session.id);
          return prev;
        });

        // Dispatch session events
        window.dispatchEvent(new CustomEvent(AppEvents.SESSION_CREATED, { detail: { session } }));
        window.dispatchEvent(
          new CustomEvent(AppEvents.ADD_ACTIVE_SESSION, {
            detail: { sessionId: session.id },
          })
        );
      } catch (error) {
        console.error('Failed to create recipe builder session:', error);
        setSessionError(errorMessage(error, 'Failed to create session'));
      } finally {
        setIsCreatingSession(false);
      }
    };

    createBuilderSession();
  }, [searchParams, setSearchParams, isCreatingSession, sessionId]);

  // Chat stream hook
  const {
    session,
    messages,
    chatState,
    handleSubmit,
    stopStreaming,
    tokenState,
    notifications,
  } = useChatStream({
    sessionId: sessionId || '',
    onStreamFinish: () => {},
  });

  // Get current recipe from form state
  const getCurrentRecipe = useCallback((): Recipe => {
    const formattedParameters = parameters.map((param) => {
      const formattedParam: Parameter = {
        key: param.key,
        input_type: param.input_type || 'string',
        requirement: param.requirement,
        description: param.description,
      };
      if (param.requirement === 'optional' && param.default) {
        formattedParam.default = param.default;
      }
      if (param.input_type === 'select' && param.options) {
        formattedParam.options = param.options.filter((opt) => opt.trim() !== '');
      }
      return formattedParam;
    });

    let responseConfig = undefined;
    if (jsonSchema && jsonSchema.trim()) {
      try {
        const parsedSchema = JSON.parse(jsonSchema);
        responseConfig = { json_schema: parsedSchema };
      } catch {
        // Invalid JSON, skip
      }
    }

    return {
      title,
      description,
      instructions,
      activities,
      prompt: prompt || undefined,
      parameters: formattedParameters,
      response: responseConfig,
    };
  }, [title, description, instructions, activities, prompt, parameters, jsonSchema]);

  const requiredFieldsAreFilled = () => {
    return title.trim() && description.trim() && (instructions.trim() || (prompt || '').trim());
  };

  // Generate deeplink when recipe changes
  useEffect(() => {
    let isCancelled = false;

    const generateLink = async () => {
      if (!requiredFieldsAreFilled()) {
        setDeeplink('');
        return;
      }

      setIsGeneratingDeeplink(true);
      try {
        const currentRecipe = getCurrentRecipe();
        const link = await generateDeepLink(currentRecipe);
        if (!isCancelled) {
          setDeeplink(link);
        }
      } catch (error) {
        console.error('Failed to generate deeplink:', error);
        if (!isCancelled) {
          setDeeplink('Error generating deeplink');
        }
      } finally {
        if (!isCancelled) {
          setIsGeneratingDeeplink(false);
        }
      }
    };

    generateLink();

    return () => {
      isCancelled = true;
    };
  }, [title, description, instructions, prompt, activities, parameters, jsonSchema, getCurrentRecipe]);

  const handleCopy = () => {
    if (!deeplink || isGeneratingDeeplink || deeplink === 'Error generating deeplink') {
      return;
    }

    navigator.clipboard
      .writeText(deeplink)
      .then(() => {
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      })
      .catch((err) => {
        console.error('Failed to copy:', err);
      });
  };

  const handleSaveRecipe = async () => {
    if (!requiredFieldsAreFilled()) {
      toastError({
        title: 'Validation Failed',
        msg: 'Please fill in all required fields.',
      });
      return;
    }

    setIsSaving(true);
    try {
      const recipe = getCurrentRecipe();
      await saveRecipe(recipe, null);

      toastSuccess({
        title: recipe.title,
        msg: 'Recipe saved successfully',
      });

      setView('recipes');
    } catch (error) {
      console.error('Failed to save recipe:', error);
      toastError({
        title: 'Save Failed',
        msg: `Failed to save recipe: ${errorMessage(error, 'Unknown error')}`,
      });
    } finally {
      setIsSaving(false);
    }
  };

  const handleSaveAndRun = async () => {
    if (!requiredFieldsAreFilled()) {
      toastError({
        title: 'Validation Failed',
        msg: 'Please fill in all required fields.',
      });
      return;
    }

    setIsSaving(true);
    try {
      const recipe = getCurrentRecipe();
      const savedRecipeId = await saveRecipe(recipe, null);

      window.electron.createChatWindow(
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        savedRecipeId
      );

      toastSuccess({
        title: recipe.title,
        msg: 'Recipe saved and launched successfully',
      });

      setView('recipes');
    } catch (error) {
      console.error('Failed to save and run recipe:', error);
      toastError({
        title: 'Save and Run Failed',
        msg: `Failed to save and run recipe: ${errorMessage(error, 'Unknown error')}`,
      });
    } finally {
      setIsSaving(false);
    }
  };

  const handleBack = () => {
    setView('recipes');
  };

  // Build a compact form state context for the AI
  const getFormContext = useCallback(() => {
    const fields: string[] = [];
    if (title) fields.push(`title="${title}"`);
    if (description) fields.push(`description="${description.substring(0, 50)}${description.length > 50 ? '...' : ''}"`);
    if (instructions) fields.push('instructions=✓');
    if (prompt) fields.push('prompt=✓');
    if (activities.length > 0) fields.push(`activities=${activities.length}`);
    if (parameters.length > 0) fields.push(`params=${parameters.length}`);

    if (fields.length === 0) return '[Form: empty]';
    return `[Form: ${fields.join(', ')}]`;
  }, [title, description, instructions, prompt, activities, parameters]);

  // Include compact form context so AI knows current state
  const handleChatSubmit = useCallback((input: UserInput) => {
    const context = getFormContext();
    // Prepend context as a brief status line
    const messageWithContext = `${context}\n${input.msg}`;
    handleSubmit({ msg: messageWithContext, images: input.images });
  }, [handleSubmit, getFormContext]);

  // Parse suggestions from the latest assistant message
  const suggestions = useMemo(() => {
    // Find the last assistant message
    for (let i = messages.length - 1; i >= 0; i--) {
      const msg = messages[i];
      if (msg.role === 'assistant') {
        const textContent = getMessageTextContent(msg);
        return parseSuggestionsFromContent(textContent);
      }
    }
    return [];
  }, [messages]);

  // Track which suggestions have been applied
  const [appliedSuggestions, setAppliedSuggestions] = useState<Set<string>>(new Set());

  // Reset applied suggestions when messages change
  useEffect(() => {
    setAppliedSuggestions(new Set());
  }, [messages.length]);

  // Handle applying a suggestion to the form
  const handleApplySuggestion = useCallback((suggestion: Suggestion) => {
    switch (suggestion.field) {
      case 'title':
        form.setFieldValue('title', suggestion.value);
        break;
      case 'description':
        form.setFieldValue('description', suggestion.value);
        break;
      case 'instructions':
        form.setFieldValue('instructions', suggestion.value);
        break;
      case 'prompt':
        form.setFieldValue('prompt', suggestion.value);
        break;
      case 'activities':
        // Parse comma-separated activities
        const activityList = suggestion.value.split(',').map((a) => a.trim()).filter(Boolean);
        form.setFieldValue('activities', activityList);
        break;
      case 'parameters':
        // Parse JSON parameters
        try {
          const params = JSON.parse(suggestion.value);
          if (Array.isArray(params)) {
            form.setFieldValue('parameters', params);
          }
        } catch {
          console.error('Failed to parse parameters JSON');
        }
        break;
    }
    // Mark as applied
    setAppliedSuggestions((prev) => new Set(prev).add(`${suggestion.field}:${suggestion.value}`));
  }, [form]);

  // Handle applying all suggestions at once
  const handleApplyAll = useCallback(() => {
    const newApplied = new Set(appliedSuggestions);
    for (const suggestion of suggestions) {
      switch (suggestion.field) {
        case 'title':
          form.setFieldValue('title', suggestion.value);
          break;
        case 'description':
          form.setFieldValue('description', suggestion.value);
          break;
        case 'instructions':
          form.setFieldValue('instructions', suggestion.value);
          break;
        case 'prompt':
          form.setFieldValue('prompt', suggestion.value);
          break;
        case 'activities':
          const activityList = suggestion.value.split(',').map((a) => a.trim()).filter(Boolean);
          form.setFieldValue('activities', activityList);
          break;
        case 'parameters':
          try {
            const params = JSON.parse(suggestion.value);
            if (Array.isArray(params)) {
              form.setFieldValue('parameters', params);
            }
          } catch {
            // Skip invalid JSON
          }
          break;
      }
      newApplied.add(`${suggestion.field}:${suggestion.value}`);
    }
    setAppliedSuggestions(newApplied);
  }, [form, suggestions, appliedSuggestions]);

  // Check if all suggestions have been applied
  const allSuggestionsApplied = useMemo(() => {
    return suggestions.every((s) => appliedSuggestions.has(`${s.field}:${s.value}`));
  }, [suggestions, appliedSuggestions]);

  // Render loading state
  if (isCreatingSession) {
    return (
      <div className="flex flex-col h-full bg-background-muted items-center justify-center">
        <LoadingGoose chatState={ChatState.LoadingConversation} />
        <p className="text-textSubtle mt-4">Creating Recipe Builder session...</p>
      </div>
    );
  }

  // Render error state
  if (sessionError) {
    return (
      <div className="flex flex-col h-full bg-background-muted items-center justify-center">
        <p className="text-red-500">{sessionError}</p>
        <Button onClick={handleBack} className="mt-4">
          Back to Recipes
        </Button>
      </div>
    );
  }

  // Render waiting for session to be created
  if (!sessionId) {
    return (
      <div className="flex flex-col h-full bg-background-muted items-center justify-center">
        <LoadingGoose chatState={ChatState.LoadingConversation} />
        <p className="text-textSubtle mt-4">Initializing...</p>
      </div>
    );
  }

  // Render loading state while useChatStream loads the session
  if (!session || chatState === ChatState.LoadingConversation) {
    return (
      <div className="flex flex-col h-full bg-background-muted items-center justify-center">
        <LoadingGoose chatState={ChatState.LoadingConversation} />
        <p className="text-textSubtle mt-4">Loading session...</p>
      </div>
    );
  }

  return (
    <div className="h-dvh flex flex-col bg-background-muted overflow-hidden">
      {/* Header */}
      <div className="flex-shrink-0 flex items-center justify-between p-4 border-b border-borderSubtle bg-background-default">
        <div className="flex items-center gap-3">
          <Button
            onClick={handleBack}
            variant="ghost"
            size="sm"
            className="p-2"
          >
            <ArrowLeft className="w-5 h-5" />
          </Button>
          <Geese className="w-6 h-6 text-iconProminent" />
          <div>
            <h1 className="text-lg font-medium text-textProminent">Recipe Builder</h1>
            <p className="text-textSubtle text-sm">Chat with AI to create your recipe</p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button
            onClick={handleSaveRecipe}
            disabled={!requiredFieldsAreFilled() || isSaving}
            variant="outline"
            size="sm"
          >
            <Save className="w-4 h-4 mr-2" />
            Save
          </Button>
          <Button
            onClick={handleSaveAndRun}
            disabled={!requiredFieldsAreFilled() || isSaving}
            variant="default"
            size="sm"
          >
            <Play className="w-4 h-4 mr-2" />
            Save & Run
          </Button>
        </div>
      </div>

      {/* Main Content - Split View */}
      <div className="flex-1 flex min-h-0">
        {/* Left: Chat */}
        <div className="w-1/2 flex flex-col min-h-0 border-r border-borderSubtle">
          {/* Chat messages - scrollable */}
          <div className="flex-1 overflow-y-auto p-4">
            {messages.length === 0 ? (
              <div className="flex flex-col items-center justify-center h-64 text-center">
                <Geese className="w-12 h-12 text-iconSubtle mb-4" />
                <p className="text-textSubtle">
                  Start a conversation to build your recipe.
                </p>
                <p className="text-textSubtle text-sm mt-2">
                  Describe what you want your recipe to do.
                </p>
              </div>
            ) : (
              <ProgressiveMessageList
                messages={messages}
                chat={{ sessionId }}
                toolCallNotifications={notifications}
                append={(text: string) => handleSubmit({ msg: text, images: [] })}
                isUserMessage={(m: Message) => m.role === 'user'}
                isStreamingMessage={chatState !== ChatState.Idle}
              />
            )}
          </div>

          {/* Suggestions panel */}
          {suggestions.length > 0 && (
            <div className="flex-shrink-0 border-t border-borderSubtle bg-background-default p-3">
              <div className="flex items-center justify-between mb-2">
                <div className="flex items-center gap-2">
                  <Sparkles className="w-4 h-4 text-yellow-500" />
                  <span className="text-sm font-medium text-textProminent">AI Suggestions</span>
                </div>
                <Button
                  variant={allSuggestionsApplied ? 'ghost' : 'default'}
                  size="sm"
                  onClick={handleApplyAll}
                  disabled={allSuggestionsApplied}
                  className={`text-xs ${allSuggestionsApplied ? 'text-green-600' : ''}`}
                >
                  {allSuggestionsApplied ? (
                    <>
                      <Check className="w-3 h-3 mr-1" />
                      All Applied
                    </>
                  ) : (
                    `Apply All (${suggestions.length})`
                  )}
                </Button>
              </div>
              <div className="flex flex-wrap gap-2">
                {suggestions.map((suggestion, index) => {
                  const key = `${suggestion.field}:${suggestion.value}`;
                  const isApplied = appliedSuggestions.has(key);
                  return (
                    <Button
                      key={index}
                      variant={isApplied ? 'ghost' : 'outline'}
                      size="sm"
                      onClick={() => handleApplySuggestion(suggestion)}
                      disabled={isApplied}
                      className={`text-xs ${isApplied ? 'text-green-600' : ''}`}
                    >
                      {isApplied ? (
                        <>
                          <Check className="w-3 h-3 mr-1" />
                          {suggestion.fieldLabel} Applied
                        </>
                      ) : (
                        <>
                          Apply {suggestion.fieldLabel}
                        </>
                      )}
                    </Button>
                  );
                })}
              </div>
            </div>
          )}

          {/* Chat input - fixed at bottom */}
          <div className="flex-shrink-0 p-4 border-t border-borderSubtle">
            <ChatInput
              sessionId={sessionId}
              handleSubmit={handleChatSubmit}
              chatState={chatState}
              onStop={stopStreaming}
              initialValue=""
              setView={setView}
              totalTokens={tokenState.totalTokens}
              accumulatedInputTokens={tokenState.accumulatedInputTokens}
              accumulatedOutputTokens={tokenState.accumulatedOutputTokens}
              droppedFiles={[]}
              onFilesProcessed={() => {}}
              messages={messages}
              disableAnimation={false}
              sessionCosts={undefined}
              toolCount={0}
            />
          </div>
        </div>

        {/* Right: Form */}
        <div className="w-1/2 flex flex-col min-h-0">
          {/* Form - scrollable */}
          <div className="flex-1 overflow-y-auto px-6 py-4">
            <RecipeFormFields form={form} />

            {/* Deep Link Display */}
            {requiredFieldsAreFilled() && (
              <div className="w-full p-4 bg-bgSubtle rounded-lg mt-6">
                <div className="flex items-center justify-between mb-2">
                  <div className="text-sm text-textSubtle">Share this recipe</div>
                  <Button
                    onClick={handleCopy}
                    variant="ghost"
                    size="sm"
                    disabled={!deeplink || isGeneratingDeeplink || deeplink === 'Error generating deeplink'}
                    className="p-2"
                  >
                    {copied ? (
                      <Check className="w-4 h-4 text-green-500" />
                    ) : (
                      <Copy className="w-4 h-4 text-iconSubtle" />
                    )}
                    <span className="ml-1 text-sm text-textSubtle">
                      {copied ? 'Copied!' : 'Copy'}
                    </span>
                  </Button>
                </div>
                <div
                  onClick={handleCopy}
                  className="text-sm truncate font-mono cursor-pointer text-textStandard"
                >
                  {isGeneratingDeeplink ? 'Generating...' : deeplink || 'Click to generate'}
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
