import {
  initializeBundledExtensions,
  syncBundledExtensions,
  addToAgentOnStartup,
} from '../components/settings/extensions';
import type { ExtensionConfig, FixedExtensionEntry } from '../components/ConfigContext';
import { toastService, ExtensionLoadingStatus } from '../toasts';
import { errorMessage } from './conversionUtils';
import { createExtensionRecoverHints } from './extensionErrorUtils';

export interface ExtensionLoadingOptions {
  getExtensions: (forceRefresh: boolean) => Promise<FixedExtensionEntry[]>;
  addExtension: (name: string, config: ExtensionConfig, enabled: boolean) => Promise<void>;
  setIsExtensionsLoading?: (loading: boolean) => void;
}

/**
 * Load extensions progressively with grouped toast notifications.
 * This is the shared logic used by both new window creation and session switching.
 */
export async function loadExtensionsProgressively(
  sessionId: string,
  options: ExtensionLoadingOptions
): Promise<void> {
  const { getExtensions, addExtension, setIsExtensionsLoading } = options;

  // Initialize or sync built-in extensions into config.yaml
  let refreshedExtensions = await getExtensions(false);

  if (refreshedExtensions.length === 0) {
    await initializeBundledExtensions(addExtension);
    refreshedExtensions = await getExtensions(false);
  } else {
    await syncBundledExtensions(refreshedExtensions, addExtension);
  }

  const enabledExtensions = refreshedExtensions.filter((ext) => ext.enabled);

  if (enabledExtensions.length === 0) {
    return;
  }

  setIsExtensionsLoading?.(true);

  const extensionStatuses: Map<string, ExtensionLoadingStatus> = new Map(
    enabledExtensions.map((ext) => [ext.name, { name: ext.name, status: 'loading' as const }])
  );

  const updateToast = (isComplete: boolean = false) => {
    toastService.extensionLoading(
      Array.from(extensionStatuses.values()),
      enabledExtensions.length,
      isComplete
    );
  };

  updateToast();

  // Load extensions in parallel and update status progressively
  const extensionLoadingPromises = enabledExtensions.map(async (extensionConfig) => {
    const extensionName = extensionConfig.name;

    try {
      await addToAgentOnStartup({
        extensionConfig,
        toastOptions: { silent: true },
        sessionId,
      });

      extensionStatuses.set(extensionName, {
        name: extensionName,
        status: 'success',
      });
      updateToast();
    } catch (error) {
      console.error(`Failed to load extension ${extensionName}:`, error);

      const errMsg = errorMessage(error);
      const recoverHints = createExtensionRecoverHints(errMsg);

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

  updateToast(true);

  setIsExtensionsLoading?.(false);
}
