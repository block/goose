import {
  initializeBundledExtensions,
  syncBundledExtensions,
} from '../components/settings/extensions';
import type { ExtensionConfig, FixedExtensionEntry } from '../components/ConfigContext';
import { updateAgentProvider, updateFromSession } from '../api';
import type { Recipe } from '../api';

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

export const initializeSystem = async (
  sessionId: string,
  provider: string,
  model: string,
  options?: {
    getExtensions?: (b: boolean) => Promise<FixedExtensionEntry[]>;
    addExtension?: (name: string, config: ExtensionConfig, enabled: boolean) => Promise<void>;
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
    await updateFromSession({
      body: {
        session_id: sessionId,
      },
      throwOnError: true,
    });

    if (!options?.getExtensions || !options?.addExtension) {
      console.warn('Extension helpers not provided in alpha mode');
      return;
    }

    // Initialize or sync built-in extensions into config.yaml
    let refreshedExtensions = await options.getExtensions(false);

    if (refreshedExtensions.length === 0) {
      await initializeBundledExtensions(options.addExtension);
    } else {
      await syncBundledExtensions(refreshedExtensions, options.addExtension);
    }
  } catch (error) {
    console.error('Failed to initialize agent:', error);
    throw error;
  }
};
