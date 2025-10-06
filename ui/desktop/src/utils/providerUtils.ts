import {
  initializeBundledExtensions,
  syncBundledExtensions,
  addToAgentOnStartup,
} from '../components/settings/extensions';
import type { ExtensionConfig, FixedExtensionEntry } from '../components/ConfigContext';
import { addSubRecipesToAgent } from '../recipe/add_sub_recipe_on_agent';
import {
  extendPrompt,
  Recipe,
  RecipeParameter,
  SubRecipe,
  updateAgentProvider,
  updateSessionConfig,
} from '../api';

// Desktop-specific system prompt extension
const desktopPrompt = `You are being accessed through the Goose Desktop application.

The user is interacting with you through a graphical user interface with the following features:
- A chat interface where messages are displayed in a conversation format
- Support for markdown formatting in your responses
- Support for code blocks with syntax highlighting
- Tool use messages are included in the chat but outputs may need to be expanded

The user can add extensions for you through the "Settings" page, which is available in the menu
on the top right of the window. There is a section on that page for extensions, and it links to
the registry.

Some extensions are builtin, such as Developer and Memory, while
3rd party extensions can be browsed at https://block.github.io/goose/v1/extensions/.
`;

// Desktop-specific system prompt extension when a bot is in play
const desktopPromptBot = `You are a helpful agent.
You are being accessed through the Goose Desktop application, pre configured with instructions as requested by a human.

The user is interacting with you through a graphical user interface with the following features:
- A chat interface where messages are displayed in a conversation format
- Support for markdown formatting in your responses
- Support for code blocks with syntax highlighting
- Tool use messages are included in the chat but outputs may need to be expanded

It is VERY IMPORTANT that you take note of the provided instructions, also check if a style of output is requested and always do your best to adhere to it.
You can also validate your output after you have generated it to ensure it meets the requirements of the user.
There may be (but not always) some tools mentioned in the instructions which you can check are available to this instance of goose (and try to help the user if they are not or find alternatives).
`;

// Helper function to extract template variables from text (matches backend logic)
export const extractTemplateVariables = (content: string): string[] => {
  const templateVarRegex = /\{\{(.*?)\}\}/g;
  const variables: string[] = [];
  let match;

  while ((match = templateVarRegex.exec(content)) !== null) {
    const variable = match[1].trim();

    if (variable && !variables.includes(variable)) {
      // Filter out complex variables that aren't valid parameter names
      // This matches the backend logic in filter_complex_variables()
      const isValid = isValidParameterName(variable);

      if (isValid) {
        variables.push(variable);
      }
    }
  }

  return variables;
};

// Helper function to check if a variable name is valid for parameters
// Matches backend regex: r"^\s*[a-zA-Z_][a-zA-Z0-9_]*\s*$"
const isValidParameterName = (variable: string): boolean => {
  const validVarRegex = /^\s*[a-zA-Z_][a-zA-Z0-9_]*\s*$/;
  return validVarRegex.test(variable);
};

// Helper function to filter recipe parameters to only show valid ones that are actually used
export const filterValidUsedParameters = (
  parameters: RecipeParameter[] | undefined,
  recipeContent: { prompt?: string; instructions?: string; activities?: string[] }
): RecipeParameter[] => {
  if (!parameters) {
    return [];
  }

  // Extract all template variables used in the recipe content
  const promptVariables = recipeContent.prompt
    ? extractTemplateVariables(recipeContent.prompt)
    : [];
  const instructionVariables = recipeContent.instructions
    ? extractTemplateVariables(recipeContent.instructions)
    : [];

  // Extract variables from activities using flatMap
  const activityVariables = recipeContent.activities?.flatMap(extractTemplateVariables) ?? [];

  const allUsedVariables = [
    ...new Set([...promptVariables, ...instructionVariables, ...activityVariables]),
  ];

  // Filter parameters to only include:
  // 1. Parameters with valid names (no spaces, dots, pipes, etc.)
  // 2. Parameters that are actually used in the recipe content
  // 3. Remove duplicates (keep first occurrence)
  const seenKeys = new Set<string>();

  return parameters.filter((param) => {
    // Check if parameter key is valid (no spaces, special characters)
    const isValid = isValidParameterName(param.key);
    if (!isValid) {
      return false;
    }

    // Check if parameter is actually used in the recipe content
    const isUsed = allUsedVariables.includes(param.key);
    if (!isUsed) {
      return false;
    }

    // Remove duplicates (keep first occurrence)
    if (seenKeys.has(param.key)) {
      return false;
    }

    seenKeys.add(param.key);
    return true;
  });
};

// Helper function to substitute parameters in text
export const substituteParameters = (text: string, params: Record<string, string>): string => {
  let substitutedText = text;

  for (const key in params) {
    // Escape special characters in the key (parameter) and match optional whitespace
    const regex = new RegExp(`{{\\s*${key.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\s*}}`, 'g');
    substitutedText = substitutedText.replace(regex, params[key]);
  }

  return substitutedText;
};

/**
 * Updates the system prompt with parameter-substituted instructions
 * This should be called after recipe parameters are collected
 */
export const updateSystemPromptWithParameters = async (
  sessionId: string,
  recipeParameters: Record<string, string>,
  recipe?: {
    instructions?: string | null;
    sub_recipes?: SubRecipe[] | null;
    parameters?: RecipeParameter[] | null;
  }
): Promise<void> => {
  const subRecipes = recipe?.sub_recipes;
  try {
    const originalInstructions = recipe?.instructions;

    if (!originalInstructions) {
      return;
    }
    // Substitute parameters in the instructions
    const substitutedInstructions = substituteParameters(originalInstructions, recipeParameters);

    // Update the system prompt with substituted instructions
    const response = await extendPrompt({
      body: {
        session_id: sessionId,
        extension: `${desktopPromptBot}\nIMPORTANT instructions for you to operate as agent:\n${substitutedInstructions}`,
      },
    });
    if (response.error) {
      console.warn(`Failed to update system prompt with parameters: ${response.error}`);
    }
  } catch (error) {
    console.error('Error updating system prompt with parameters:', error);
  }
  if (subRecipes && subRecipes?.length > 0) {
    for (const subRecipe of subRecipes) {
      if (subRecipe.values) {
        for (const key in subRecipe.values) {
          subRecipe.values[key] = substituteParameters(subRecipe.values[key], recipeParameters);
        }
      }
    }
    await addSubRecipesToAgent(sessionId, subRecipes);
  }
};

export const initializeSystem = async (
  sessionId: string,
  provider: string,
  model: string,
  options?: {
    getExtensions?: (b: boolean) => Promise<FixedExtensionEntry[]>;
    addExtension?: (name: string, config: ExtensionConfig, enabled: boolean) => Promise<void>;
    setIsExtensionsLoading?: (loading: boolean) => void;
    recipeParameters?: Record<string, string> | null;
    recipe?: Recipe;
  }
) => {
  try {
    console.log(
      'initializing agent with provider',
      provider,
      'model',
      model,
      'sessionId',
      sessionId
    );
    await updateAgentProvider({
      body: {
        session_id: sessionId,
        provider,
        model,
      },
      throwOnError: true,
    });

    if (!sessionId) {
      console.log('This will not end well');
    }

    // Get recipe - prefer from options (session metadata) over app config
    const recipe = options?.recipe || window.appConfig?.get?.('recipe');
    const recipe_instructions = (recipe as { instructions?: string })?.instructions;
    const responseConfig = (recipe as { response?: { json_schema?: unknown } })?.response;
    const subRecipes = (recipe as { sub_recipes?: SubRecipe[] })?.sub_recipes;
    const hasSubRecipes = subRecipes && subRecipes?.length > 0;
    const recipeParameters = options?.recipeParameters;

    // Determine the system prompt
    let prompt = desktopPrompt;

    // If we have recipe instructions, add them to the system prompt with parameter substitution
    if (recipe_instructions) {
      const substitutedInstructions = recipeParameters
        ? substituteParameters(recipe_instructions, recipeParameters)
        : recipe_instructions;

      prompt = `${desktopPromptBot}\nIMPORTANT instructions for you to operate as agent:\n${substitutedInstructions}`;
    }

    // Extend the system prompt with desktop-specific information
    await extendPrompt({
      body: {
        session_id: sessionId,
        extension: prompt,
      },
    });

    if (hasSubRecipes) {
      let finalSubRecipes = subRecipes;

      // If we have parameters, substitute them in sub-recipe values
      if (recipeParameters) {
        finalSubRecipes = subRecipes.map((subRecipe) => ({
          ...subRecipe,
          values: subRecipe.values
            ? Object.fromEntries(
                Object.entries(subRecipe.values).map(([key, value]) => [
                  key,
                  substituteParameters(value, recipeParameters),
                ])
              )
            : subRecipe.values,
        }));
      }

      await addSubRecipesToAgent(sessionId, finalSubRecipes);
    }

    // Configure session with response config if present
    if (responseConfig?.json_schema) {
      const sessionConfigResponse = await updateSessionConfig({
        body: {
          session_id: sessionId,
          response: responseConfig,
        },
      });
      if (sessionConfigResponse.error) {
        console.warn(`Failed to configure session: ${sessionConfigResponse.error}`);
      }
    }

    if (!options?.getExtensions || !options?.addExtension) {
      console.warn('Extension helpers not provided in alpha mode');
      return;
    }

    // Initialize or sync built-in extensions into config.yaml
    let refreshedExtensions = await options.getExtensions(false);

    if (refreshedExtensions.length === 0) {
      await initializeBundledExtensions(options.addExtension);
      refreshedExtensions = await options.getExtensions(false);
    } else {
      await syncBundledExtensions(refreshedExtensions, options.addExtension);
    }

    // Add enabled extensions to agent in parallel
    const enabledExtensions = refreshedExtensions.filter((ext) => ext.enabled);

    options?.setIsExtensionsLoading?.(true);

    const extensionLoadingPromises = enabledExtensions.map(async (extensionConfig) => {
      const extensionName = extensionConfig.name;

      try {
        await addToAgentOnStartup({
          addToConfig: options.addExtension!,
          extensionConfig,
          toastOptions: { silent: false },
          sessionId,
        });
      } catch (error) {
        console.error(`Failed to load extension ${extensionName}:`, error);
      }
    });

    await Promise.allSettled(extensionLoadingPromises);
    options?.setIsExtensionsLoading?.(false);
  } catch (error) {
    console.error('Failed to initialize agent:', error);
    options?.setIsExtensionsLoading?.(false);
    throw error;
  }
};
