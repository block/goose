import {
  initializeBundledExtensions,
  syncBundledExtensions,
  addToAgentOnStartup,
} from '../components/settings/extensions';
import type { ExtensionConfig, FixedExtensionEntry } from '../components/ConfigContext';
import { Recipe, updateAgentProvider, updateFromSession } from '../api';
import { toastService, ExtensionLoadingStatus } from '../toasts';
import { errorMessage } from './conversionUtils';
import { createExtensionRecoverHints } from './extensionErrorUtils';

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
      refreshedExtensions = await options.getExtensions(false);
    } else {
      await syncBundledExtensions(refreshedExtensions, options.addExtension);
    }

    // Add enabled extensions to agent in parallel
    const enabledExtensions = refreshedExtensions.filter((ext) => ext.enabled);

    if (enabledExtensions.length === 0) {
      return;
    }

    options?.setIsExtensionsLoading?.(true);

    // Initialize extension status tracking
    const extensionStatuses: Map<string, ExtensionLoadingStatus> = new Map(
      enabledExtensions.map((ext) => [ext.name, { name: ext.name, status: 'loading' as const }])
    );

    // Show initial loading toast
    const updateToast = (isComplete: boolean = false) => {
      toastService.extensionLoading(
        Array.from(extensionStatuses.values()),
        enabledExtensions.length,
        isComplete
      );
    };

    updateToast();

    // Load extensions in parallel and update status
    const extensionLoadingPromises = enabledExtensions.map(async (extensionConfig) => {
      const extensionName = extensionConfig.name;

      try {
        await addToAgentOnStartup({
          extensionConfig,
          toastOptions: { silent: true }, // Silent since we're using grouped notification
          sessionId,
        });

        // Update status to success
        extensionStatuses.set(extensionName, {
          name: extensionName,
          status: 'success',
        });
        updateToast();
      } catch (error) {
        console.error(`Failed to load extension ${extensionName}:`, error);

        // Extract error message using shared utility
        const errMsg = errorMessage(error);

        // Create recovery hints for "Ask goose" button
        const recoverHints = createExtensionRecoverHints(errMsg);

        // Update status to error
        extensionStatuses.set(extensionName, {
          name: extensionName,
          status: 'error',
          error: errMsg,
          recoverHints,
        });
        updateToast();
      }
    });

    await Promise.allSettled(extensionLoadingPromises);

    // Show final completion toast
    updateToast(true);

    options?.setIsExtensionsLoading?.(false);
  } catch (error) {
    console.error('Failed to initialize agent:', error);
    options?.setIsExtensionsLoading?.(false);
    throw error;
  }
};
