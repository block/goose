import { getApiUrl, getSecretKey } from '../config';
import { FullExtensionConfig, loadAndAddStoredExtensions } from '../extensions';
import { GOOSE_PROVIDER, GOOSE_MODEL } from '../env_vars';
import { Model } from '../components/settings/models/ModelContext';
import { gooseModels } from '../components/settings/models/GooseModels';
import { initializeAgent } from '../agent';
import { settingsV2Enabled } from '../flags';
import {
  initializeBundledExtensions,
  syncBundledExtensions,
  addToAgentOnStartup,
} from '../components/settings_v2/extensions';
import { extractExtensionConfig } from '../components/settings_v2/extensions/utils';
import type { ExtensionConfig, FixedExtensionEntry } from '../components/ConfigContext';
// TODO: remove when removing migration logic
import { toastService } from '../toasts';
import { ExtensionQuery, addExtension as apiAddExtension } from '../api';

interface AppConfig {
  GOOSE_PROVIDER?: string;
  [key: string]: unknown;
}

export function getStoredProvider(config: AppConfig): string | null {
  return config.GOOSE_PROVIDER || localStorage.getItem(GOOSE_PROVIDER);
}

export function getStoredModel(): string | null {
  const storedModel = localStorage.getItem('GOOSE_MODEL'); // Adjust key name if necessary

  if (storedModel) {
    try {
      const modelInfo: Model = JSON.parse(storedModel);
      return modelInfo.name || null; // Return name if it exists, otherwise null
    } catch (error) {
      console.error('Error parsing GOOSE_MODEL from local storage:', error);
      return null; // Return null if parsing fails
    }
  }

  return null; // Return null if storedModel is not found
}

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

/**
 * Migrates extensions from localStorage to config.yaml (settings v2)
 * This function handles the migration from settings v1 to v2 by:
 * 1. Reading extensions from localStorage
 * 2. Adding non-builtin extensions to config.yaml
 * 3. Marking the migration as complete
 *
 * NOTE: This logic can be removed eventually when enough versions have passed
 * We leave the existing user settings in localStorage, in case users downgrade
 * or things need to be reverted.
 *
 * @param addExtension Function to add extension to config.yaml
 */
export const migrateExtensionsToSettingsV2 = async () => {
  console.log('need to perform extension migration');

  const userSettingsStr = localStorage.getItem('user_settings');
  let localStorageExtensions: FullExtensionConfig[] = [];

  try {
    if (userSettingsStr) {
      const userSettings = JSON.parse(userSettingsStr);
      localStorageExtensions = userSettings.extensions ?? [];
    }
  } catch (error) {
    console.error('Failed to parse user settings:', error);
  }

  const migrationErrors: { name: string; error: unknown }[] = [];

  for (const extension of localStorageExtensions) {
    // NOTE: skip migrating builtin types since there was a format change
    // instead we rely on initializeBundledExtensions & syncBundledExtensions
    // to handle updating / creating the new builtins to the config.yaml
    // For all other extension types we migrate them to config.yaml
    if (extension.type !== 'builtin') {
      console.log(`Migrating extension ${extension.name} to config.yaml`);
      try {
        // manually import apiAddExtension to set throwOnError true
        const query: ExtensionQuery = {
          name: extension.name,
          config: extension,
          enabled: extension.enabled,
        };
        await apiAddExtension({
          body: query,
          throwOnError: true,
        });
      } catch (err) {
        console.error(`Failed to migrate extension ${extension.name}:`, err);
        migrationErrors.push({
          name: extension.name,
          error: `failed migration with ${JSON.stringify(err)}`,
        });
      }
    }
  }

  if (migrationErrors.length === 0) {
    localStorage.setItem('configVersion', '2');
    console.log('Extension migration complete. Config version set to 2.');
  } else {
    const errorSummaryStr = migrationErrors
      .map(({ name, error }) => `- ${name}: ${JSON.stringify(error)}`)
      .join('\n');
    toastService.error({
      title: 'Config Migration Error',
      msg: 'There was a problem updating your config file',
      traceback: errorSummaryStr,
    });
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

    // This will go away after the release of settings v2 as this is now handled in config.yaml
    if (!settingsV2Enabled) {
      // Sync the model state with React
      const syncedModel = syncModelWithAgent(provider, model);
      console.log('Model synced with React state:', syncedModel);
    }

    // Get botConfig directly here
    const botConfig = window.appConfig?.get?.('botConfig');
    const botPrompt = botConfig?.instructions;

    // Extend the system prompt with desktop-specific information
    const response = await fetch(getApiUrl('/agent/prompt'), {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': getSecretKey(),
      },
      body: JSON.stringify({
        extension: botPrompt
          ? `${desktopPromptBot}\nIMPORTANT instructions for you to operate as agent:\n${botPrompt}`
          : desktopPrompt,
      }),
    });

    if (!response.ok) {
      console.warn(`Failed to extend system prompt: ${response.statusText}`);
    } else {
      console.log('Extended system prompt with desktop-specific information');
      if (botPrompt) {
        console.log('Added custom bot prompt to system prompt');
      }
    }

    if (settingsV2Enabled) {
      if (!options?.getExtensions || !options?.addExtension) {
        console.warn('Extension helpers not provided in alpha mode');
        return;
      }

      // NOTE: remove when we want to stop migration logic
      // Check if we need to migrate extensions from localStorage to config.yaml
      const configVersion = localStorage.getItem('configVersion');
      const shouldMigrateExtensions = !configVersion || parseInt(configVersion, 10) < 2;

      console.log(`shouldMigrateExtensions is ${shouldMigrateExtensions}`);
      if (shouldMigrateExtensions) {
        await migrateExtensionsToSettingsV2();
      }

      /* NOTE:
       * If we've migrated and this is a version update, refreshedExtensions should be > 0
       *  and we'll want to syncBundledExtensions to ensure any new extensions are added.
       * Otherwise if the user has never opened goose - refreshedExtensions will be 0
       *  and we want to fall into the case to initializeBundledExtensions.
       */

      // Initialize or sync built-in extensions into config.yaml
      let refreshedExtensions = await options.getExtensions(false);

      if (refreshedExtensions.length === 0) {
        await initializeBundledExtensions(options.addExtension);
        refreshedExtensions = await options.getExtensions(false);
      } else {
        await syncBundledExtensions(refreshedExtensions, options.addExtension);
      }

      // enable all extensions in parallel
      await Promise.all(
        refreshedExtensions
          .filter((e) => e.enabled)
          .map((extensionEntry) => {
            const extensionConfig = extractExtensionConfig(extensionEntry);
            return addToAgentOnStartup({ addToConfig: options.addExtension, extensionConfig });
          })
      );
    } else {
      loadAndAddStoredExtensions().catch((error) => {
        console.error('Failed to load and add stored extension configs:', error);
      });
    }
  } catch (error) {
    console.error('Failed to initialize agent:', error);
    throw error;
  }
};

// This function ensures the agent initialization values and React model state stay in sync
const syncModelWithAgent = (provider: string, modelName: string): Model | null => {
  console.log('Syncing model state with agent:', { provider, modelName });

  // First, try to find a matching model in our predefined list
  let matchingModel = gooseModels.find(
    (m) => m.name === modelName && m.provider.toLowerCase() === provider.toLowerCase()
  );

  // If no match by exact name and provider, try just by provider
  if (!matchingModel) {
    matchingModel = gooseModels.find((m) => m.provider.toLowerCase() === provider.toLowerCase());

    if (matchingModel) {
      console.log('Found model by provider only:', matchingModel);
    }
  }

  // If still no match, create a custom model
  if (!matchingModel) {
    console.log('No matching model found, creating custom model');
    matchingModel = {
      id: Date.now(),
      name: modelName,
      provider: provider,
      alias: `${provider} - ${modelName}`,
    };
  }

  // Update localStorage with the model
  if (matchingModel) {
    localStorage.setItem(GOOSE_PROVIDER, matchingModel.provider.toLowerCase());
    localStorage.setItem(GOOSE_MODEL, JSON.stringify(matchingModel));

    return matchingModel;
  }

  return null;
};
