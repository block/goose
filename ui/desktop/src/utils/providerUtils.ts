import { getApiUrl } from '../config';
import { initializeAgent } from '../agent';
import {
  initializeBundledExtensions,
  syncBundledExtensions,
  addToAgentOnStartup,
} from '../components/settings/extensions';
import { extractExtensionConfig } from '../components/settings/extensions/utils';
import type { ExtensionConfig, FixedExtensionEntry } from '../components/ConfigContext';
import { RecipeParameter, SubRecipe, updateSessionConfig, extendPrompt } from '../api';
import { addSubRecipesToAgent } from '../recipe/add_sub_recipe_on_agent';

export interface Provider {
  id: string; // Lowercase key (e.g., "openai")
  name: string; // Provider name (e.g., "OpenAI")
  description: string; // Description of the provider
  models: string[]; // List of supported models
  requiredKeys: string[]; // List of required keys
}

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

// Helper function to substitute parameters in text
const substituteParameters = (text: string, params: Record<string, string>): string => {
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
  recipeParameters: Record<string, string>,
  recipeConfig?: {
    instructions?: string | null;
    sub_recipes?: SubRecipe[] | null;
    parameters?: RecipeParameter[] | null;
  }
): Promise<void> => {
  const subRecipes = recipeConfig?.sub_recipes;
  try {
    const originalInstructions = recipeConfig?.instructions;

    if (!originalInstructions) {
      return;
    }
    // Substitute parameters in the instructions
    const substitutedInstructions = substituteParameters(originalInstructions, recipeParameters);

    // Update the system prompt with substituted instructions
    const response = await extendPrompt({
      body: {
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
    await addSubRecipesToAgent(subRecipes);
  }
};

export const initializeSystem = async (
  provider: string,
  model: string,
  options?: {
    getExtensions?: (b: boolean) => Promise<FixedExtensionEntry[]>;
    addExtension?: (name: string, config: ExtensionConfig, enabled: boolean) => Promise<void>;
  }
) => {
  try {
    console.log('initializing agent with provider', provider, 'model', model);
    await initializeAgent({ provider, model });

    // Get recipeConfig directly here
    const recipeConfig = window.appConfig?.get?.('recipe');
    const recipe_instructions = (recipeConfig as { instructions?: string })?.instructions;
    const responseConfig = (recipeConfig as { response?: { json_schema?: unknown } })?.response;
    const subRecipes = (recipeConfig as { sub_recipes?: SubRecipe[] })?.sub_recipes;
    const parameters = (recipeConfig as { parameters?: RecipeParameter[] })?.parameters;
    const hasParameters = parameters && parameters?.length > 0;
    const hasSubRecipes = subRecipes && subRecipes?.length > 0;
    let prompt = desktopPrompt;
    if (!hasParameters && recipe_instructions) {
      prompt = `${desktopPromptBot}\nIMPORTANT instructions for you to operate as agent:\n${recipe_instructions}`;
    }
    // Extend the system prompt with desktop-specific information
    const response = await fetch(getApiUrl('/agent/prompt'), {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': await window.electron.getSecretKey(),
      },
      body: JSON.stringify({
        extension: prompt,
      }),
    });
    if (!response.ok) {
      console.warn(`Failed to extend system prompt: ${response.statusText}`);
    } else {
      console.log('Extended system prompt with desktop-specific information');
    }
    if (!hasParameters && hasSubRecipes) {
      await addSubRecipesToAgent(subRecipes);
    }
    // Configure session with response config if present
    if (responseConfig?.json_schema) {
      const sessionConfigResponse = await updateSessionConfig({
        body: {
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

    const extensionLoadingPromises = enabledExtensions.map(async (extensionEntry) => {
      const extensionConfig = extractExtensionConfig(extensionEntry);
      const extensionName = extensionConfig.name;

      try {
        await addToAgentOnStartup({ addToConfig: options.addExtension!, extensionConfig });
      } catch (error) {
        console.error(`Failed to load extension ${extensionName}:`, error);
      }
    });

    await Promise.allSettled(extensionLoadingPromises);
  } catch (error) {
    console.error('Failed to initialize agent:', error);
    throw error;
  }
};
