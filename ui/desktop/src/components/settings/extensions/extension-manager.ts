import type { ExtensionConfig } from '../../../api/types.gen';
import { toastService, ToastServiceOptions } from '../../../toasts';
import { addToAgent, removeFromAgent, sanitizeName } from './agent-api';
import {
  trackExtensionAdded,
  trackExtensionEnabled,
  trackExtensionDisabled,
  trackExtensionDeleted,
  getErrorType,
} from '../../../utils/analytics';

function isBuiltinExtension(config: ExtensionConfig): boolean {
  return config.type === 'builtin';
}

interface ActivateExtensionProps {
  addToConfig: (name: string, extensionConfig: ExtensionConfig, enabled: boolean) => Promise<void>;
  extensionConfig: ExtensionConfig;
  sessionId: string;
}

type ExtensionError = {
  message?: string;
  code?: number;
  name?: string;
  stack?: string;
};

type RetryOptions = {
  retries?: number;
  delayMs?: number;
  shouldRetry?: (error: ExtensionError, attempt: number) => boolean;
  backoffFactor?: number; // multiplier for exponential backoff
};

async function retryWithBackoff<T>(fn: () => Promise<T>, options: RetryOptions = {}): Promise<T> {
  const { retries = 3, delayMs = 1000, backoffFactor = 1.5, shouldRetry = () => true } = options;

  let attempt = 0;
  let lastError: ExtensionError = new Error('Unknown error');

  while (attempt <= retries) {
    try {
      return await fn();
    } catch (err) {
      lastError = err as ExtensionError;
      attempt++;

      if (attempt > retries || !shouldRetry(lastError, attempt)) {
        break;
      }

      const waitTime = delayMs * Math.pow(backoffFactor, attempt - 1);
      console.warn(`Retry attempt ${attempt} failed. Retrying in ${waitTime}ms...`, err);
      await new Promise((res) => setTimeout(res, waitTime));
    }
  }

  throw lastError;
}

/**
 * Activates an extension by adding it to both the config system and the API.
 * @param props The extension activation properties
 * @returns Promise that resolves when activation is complete
 */
export async function activateExtension({
  addToConfig,
  extensionConfig,
  sessionId,
}: ActivateExtensionProps): Promise<void> {
  const isBuiltin = isBuiltinExtension(extensionConfig);

  try {
    // AddToAgent
    await addToAgent(extensionConfig, sessionId, true);
  } catch (error) {
    console.error('Failed to add extension to agent:', error);
    await addToConfig(extensionConfig.name, extensionConfig, false);
    trackExtensionAdded(extensionConfig.name, false, getErrorType(error), isBuiltin);
    throw error;
  }

  try {
    await addToConfig(extensionConfig.name, extensionConfig, true);
    trackExtensionAdded(extensionConfig.name, true, undefined, isBuiltin);
  } catch (error) {
    console.error('Failed to add extension to config:', error);
    // remove from Agent
    try {
      await removeFromAgent(extensionConfig.name, sessionId, true);
    } catch (removeError) {
      console.error('Failed to remove extension from agent after config failure:', removeError);
    }
    trackExtensionAdded(extensionConfig.name, false, getErrorType(error), isBuiltin);
    // Rethrow the error to inform the caller
    throw error;
  }
}

interface AddToAgentOnStartupProps {
  extensionConfig: ExtensionConfig;
  toastOptions?: ToastServiceOptions;
  sessionId: string;
}

/**
 * Adds an extension to the agent during application startup with retry logic
 *
 * TODO(Douwe): Delete this after basecamp lands
 */
export async function addToAgentOnStartup({
  extensionConfig,
  sessionId,
  toastOptions,
}: AddToAgentOnStartupProps): Promise<void> {
  const showToast = !toastOptions?.silent;

  // Errors are caught by the grouped notification in providerUtils.ts
  // Individual error toasts are suppressed during startup (showToast=false)
  await retryWithBackoff(() => addToAgent(extensionConfig, sessionId, showToast), {
    retries: 3,
    delayMs: 1000,
    shouldRetry: (error: ExtensionError) =>
      !!error.message &&
      (error.message.includes('428') ||
        error.message.includes('Precondition Required') ||
        error.message.includes('Agent is not initialized')),
  });
}

interface UpdateExtensionProps {
  enabled: boolean;
  addToConfig: (name: string, extensionConfig: ExtensionConfig, enabled: boolean) => Promise<void>;
  removeFromConfig: (name: string) => Promise<void>;
  extensionConfig: ExtensionConfig;
  originalName?: string;
  sessionId?: string;
}

export async function updateExtension({
  enabled,
  addToConfig,
  removeFromConfig,
  extensionConfig,
  originalName,
  sessionId,
}: UpdateExtensionProps) {
  const sanitizedNewName = sanitizeName(extensionConfig.name);
  const sanitizedOriginalName = originalName ? sanitizeName(originalName) : undefined;
  const nameChanged = sanitizedOriginalName && sanitizedOriginalName !== sanitizedNewName;

  if (nameChanged) {
    if (sessionId) {
      try {
        await removeFromAgent(originalName!, sessionId, false);
      } catch (error) {
        console.error('Failed to remove old extension from agent during rename:', error);
        // Continue with the process even if agent removal fails
      }
    }

    try {
      await removeFromConfig(originalName!);
    } catch (error) {
      console.error('Failed to remove old extension from config during rename:', error);
      throw error;
    }

    const sanitizedExtensionConfig = {
      ...extensionConfig,
      name: sanitizedNewName,
    };

    if (enabled && sessionId) {
      try {
        await addToAgent(sanitizedExtensionConfig, sessionId, false);
      } catch (error) {
        console.error('[updateExtension]: Failed to add renamed extension to agent:', error);
        throw error;
      }
    }

    try {
      await addToConfig(sanitizedNewName, sanitizedExtensionConfig, enabled);
    } catch (error) {
      console.error('[updateExtension]: Failed to add renamed extension to config:', error);
      throw error;
    }

    toastService.configure({ silent: false });
    toastService.success({
      title: `Update extension`,
      msg: `Successfully updated ${sanitizedNewName} extension`,
    });
  } else {
    const sanitizedExtensionConfig = {
      ...extensionConfig,
      name: sanitizedNewName,
    };

    if (enabled && sessionId) {
      try {
        await addToAgent(sanitizedExtensionConfig, sessionId, false);
      } catch (error) {
        console.error('[updateExtension]: Failed to add extension to agent during update:', error);
        throw error;
      }

      // Then add to config
      try {
        await addToConfig(sanitizedNewName, sanitizedExtensionConfig, enabled);
      } catch (error) {
        console.error('[updateExtension]: Failed to update extension in config:', error);
        throw error;
      }

      toastService.success({
        title: `Update extension`,
        msg: `Successfully updated ${sanitizedNewName} extension`,
      });
    } else {
      try {
        await addToConfig(sanitizedNewName, sanitizedExtensionConfig, enabled);
      } catch (error) {
        console.error('[updateExtension]: Failed to update extension in config:', error);
        throw error;
      }

      toastService.success({
        title: `Update extension`,
        msg: `Successfully updated ${sanitizedNewName} extension`,
      });
    }
  }
}

interface ToggleExtensionProps {
  toggle: 'toggleOn' | 'toggleOff';
  extensionConfig: ExtensionConfig;
  addToConfig: (name: string, extensionConfig: ExtensionConfig, enabled: boolean) => Promise<void>;
  toastOptions?: ToastServiceOptions;
  sessionId: string;
}

/**
 * Toggles an extension between enabled and disabled states
 */
export async function toggleExtension({
  toggle,
  extensionConfig,
  addToConfig,
  toastOptions = {},
  sessionId,
}: ToggleExtensionProps) {
  const isBuiltin = isBuiltinExtension(extensionConfig);

  // disabled to enabled
  if (toggle == 'toggleOn') {
    try {
      // add to agent with toast options
      await addToAgent(extensionConfig, sessionId, !toastOptions?.silent);
    } catch (error) {
      console.error('Error adding extension to agent. Attempting to toggle back off.');
      trackExtensionEnabled(extensionConfig.name, false, getErrorType(error), isBuiltin);
      try {
        await toggleExtension({
          toggle: 'toggleOff',
          extensionConfig,
          addToConfig,
          toastOptions: { silent: true }, // otherwise we will see a toast for removing something that was never added
          sessionId,
        });
      } catch (toggleError) {
        console.error('Failed to toggle extension off after agent error:', toggleError);
      }
      throw error;
    }

    // update the config
    try {
      await addToConfig(extensionConfig.name, extensionConfig, true);
      trackExtensionEnabled(extensionConfig.name, true, undefined, isBuiltin);
    } catch (error) {
      console.error('Failed to update config after enabling extension:', error);
      trackExtensionEnabled(extensionConfig.name, false, getErrorType(error), isBuiltin);
      // remove from agent
      try {
        await removeFromAgent(extensionConfig.name, sessionId, !toastOptions?.silent);
      } catch (removeError) {
        console.error('Failed to remove extension from agent after config failure:', removeError);
      }
      throw error;
    }
  } else if (toggle == 'toggleOff') {
    // enabled to disabled
    let agentRemoveError = null;
    try {
      await removeFromAgent(extensionConfig.name, sessionId, !toastOptions?.silent);
    } catch (error) {
      // note there was an error, but attempt to remove from config anyway
      console.error('Error removing extension from agent', extensionConfig.name, error);
      agentRemoveError = error;
    }

    // update the config
    try {
      await addToConfig(extensionConfig.name, extensionConfig, false);
      if (agentRemoveError) {
        trackExtensionDisabled(
          extensionConfig.name,
          false,
          getErrorType(agentRemoveError),
          isBuiltin
        );
      } else {
        trackExtensionDisabled(extensionConfig.name, true, undefined, isBuiltin);
      }
    } catch (error) {
      console.error('Error removing extension from config', extensionConfig.name, 'Error:', error);
      trackExtensionDisabled(extensionConfig.name, false, getErrorType(error), isBuiltin);
      throw error;
    }

    // If we had an error removing from agent but succeeded updating config, still throw the original error
    if (agentRemoveError) {
      throw agentRemoveError;
    }
  }
}

interface DeleteExtensionProps {
  name: string;
  removeFromConfig: (name: string) => Promise<void>;
  sessionId?: string;
  extensionConfig?: ExtensionConfig;
}

/**
 * Deletes an extension completely from both agent and config
 */
export async function deleteExtension({
  name,
  removeFromConfig,
  sessionId,
  extensionConfig,
}: DeleteExtensionProps) {
  const isBuiltin = extensionConfig ? isBuiltinExtension(extensionConfig) : false;

  let agentRemoveError = null;
  if (sessionId) {
    try {
      await removeFromAgent(name, sessionId, true);
    } catch (error) {
      console.error('Failed to remove extension from agent during deletion:', error);
      agentRemoveError = error;
    }
  }

  try {
    await removeFromConfig(name);
    if (agentRemoveError) {
      trackExtensionDeleted(name, false, getErrorType(agentRemoveError), isBuiltin);
    } else {
      trackExtensionDeleted(name, true, undefined, isBuiltin);
    }
  } catch (error) {
    console.error(
      'Failed to remove extension from config after removing from agent. Error:',
      error
    );
    trackExtensionDeleted(name, false, getErrorType(error), isBuiltin);
    throw error;
  }

  if (agentRemoveError) {
    throw agentRemoveError;
  }
}

interface ToggleExtensionDefaultProps {
  toggle: 'toggleOn' | 'toggleOff';
  extensionConfig: ExtensionConfig;
  addToConfig: (name: string, extensionConfig: ExtensionConfig, enabled: boolean) => Promise<void>;
}

export async function toggleExtensionDefault({
  toggle,
  extensionConfig,
  addToConfig,
}: ToggleExtensionDefaultProps) {
  const isBuiltin = isBuiltinExtension(extensionConfig);
  const enabled = toggle === 'toggleOn';

  try {
    await addToConfig(extensionConfig.name, extensionConfig, enabled);
    if (enabled) {
      trackExtensionEnabled(extensionConfig.name, true, undefined, isBuiltin);
    } else {
      trackExtensionDisabled(extensionConfig.name, true, undefined, isBuiltin);
    }
    toastService.success({
      title: extensionConfig.name,
      msg: enabled ? 'Extension enabled as default' : 'Extension disabled as default',
    });
  } catch (error) {
    console.error('Failed to update extension default in config:', error);
    if (enabled) {
      trackExtensionEnabled(extensionConfig.name, false, getErrorType(error), isBuiltin);
    } else {
      trackExtensionDisabled(extensionConfig.name, false, getErrorType(error), isBuiltin);
    }
    toastService.error({
      title: extensionConfig.name,
      msg: 'Failed to update extension default',
    });
    throw error;
  }
}

interface ActivateExtensionDefaultProps {
  addToConfig: (name: string, extensionConfig: ExtensionConfig, enabled: boolean) => Promise<void>;
  extensionConfig: ExtensionConfig;
}

export async function activateExtensionDefault({
  addToConfig,
  extensionConfig,
}: ActivateExtensionDefaultProps): Promise<void> {
  const isBuiltin = isBuiltinExtension(extensionConfig);

  try {
    await addToConfig(extensionConfig.name, extensionConfig, true);
    trackExtensionAdded(extensionConfig.name, true, undefined, isBuiltin);
    toastService.success({
      title: extensionConfig.name,
      msg: 'Extension added as default',
    });
  } catch (error) {
    console.error('Failed to add extension to config:', error);
    trackExtensionAdded(extensionConfig.name, false, getErrorType(error), isBuiltin);
    toastService.error({
      title: extensionConfig.name,
      msg: 'Failed to add extension',
    });
    throw error;
  }
}
