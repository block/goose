import type { ExtensionConfig } from '../../../api/types.gen';
import { toastService, ToastServiceOptions } from '../../../toasts';
import { addToAgent, removeFromAgent } from './agent-api';
import { upsertConfig } from '../../../api';

// TODO: unify config.yaml and the agent /extensions/add API's notion of env vars
export type AgentExtensionConfig = ExtensionConfig & {
  env_keys?: string[];
};

interface ActivateExtensionProps {
  addToConfig: (name: string, extensionConfig: ExtensionConfig, enabled: boolean) => Promise<void>;
  extensionConfig: ExtensionConfig;
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
  let lastError: ExtensionError;

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
}: ActivateExtensionProps): Promise<void> {
  try {
    // AddToAgent
    await addToAgent(extensionConfig, { silent: false });
  } catch (error) {
    console.error('Failed to add extension to agent:', error);
    // add to config with enabled = false
    await addToConfig(extensionConfig.name, extensionConfig, false);
    // Rethrow the error to inform the caller
    throw error;
  }

  // Then add to config
  try {
    await addToConfig(extensionConfig.name, extensionConfig, true);
  } catch (error) {
    console.error('Failed to add extension to config:', error);
    // remove from Agent
    try {
      await removeFromAgent(extensionConfig.name);
    } catch (removeError) {
      console.error('Failed to remove extension from agent after config failure:', removeError);
    }
    // Rethrow the error to inform the caller
    throw error;
  }
}

interface AddToAgentOnStartupProps {
  addToConfig: (name: string, extensionConfig: ExtensionConfig, enabled: boolean) => Promise<void>;
  extensionConfig: ExtensionConfig;
}

/**
 * Adds an extension to the agent during application startup with retry logic
 */
export async function addToAgentOnStartup({
  addToConfig,
  extensionConfig,
}: AddToAgentOnStartupProps): Promise<void> {
  try {
    await retryWithBackoff(() => addToAgent(extensionConfig, { silent: true }), {
      retries: 3,
      delayMs: 1000,
      shouldRetry: (error: ExtensionError) =>
        error.message &&
        (error.message.includes('428') ||
          error.message.includes('Precondition Required') ||
          error.message.includes('Agent is not initialized')),
    });
  } catch (finalError) {
    toastService.configure({ silent: false });
    toastService.error({
      title: extensionConfig.name,
      msg: 'Extension failed to start and will be disabled.',
      traceback: finalError as Error,
    });

    try {
      await toggleExtension({
        toggle: 'toggleOff',
        extensionConfig,
        addToConfig,
        toastOptions: { silent: true },
      });
    } catch (toggleErr) {
      console.error('Failed to toggle off after error:', toggleErr);
    }
  }
}

interface UpdateExtensionProps {
  enabled: boolean;
  addToConfig: (name: string, extensionConfig: ExtensionConfig, enabled: boolean) => Promise<void>;
  extensionConfig: ExtensionConfig;
}

/**
 * Updates an extension configuration without changing its enabled state
 */
export async function updateExtension({
  enabled,
  addToConfig,
  extensionConfig,
}: UpdateExtensionProps) {
  if (enabled) {
    try {
      // AddToAgent
      await addToAgent(extensionConfig);
    } catch (error) {
      console.error('[updateExtension]: Failed to add extension to agent during update:', error);
      // Failed to add to agent -- show that error to user and do not update the config file
      throw error;
    }

    // Then add to config
    try {
      await addToConfig(extensionConfig.name, extensionConfig, enabled);
    } catch (error) {
      console.error('[updateExtension]: Failed to update extension in config:', error);
      throw error;
    }
  } else {
    try {
      await addToConfig(extensionConfig.name, extensionConfig, enabled);
    } catch (error) {
      console.error('[updateExtension]: Failed to update disabled extension in config:', error);
      throw error;
    }
    // show a toast that it was successfully updated
    toastService.success({
      title: `Update extension`,
      msg: `Successfully updated ${extensionConfig.name} extension`,
    });
  }
}

interface ToggleExtensionProps {
  toggle: 'toggleOn' | 'toggleOff';
  extensionConfig: ExtensionConfig;
  addToConfig: (name: string, extensionConfig: ExtensionConfig, enabled: boolean) => Promise<void>;
  toastOptions?: ToastServiceOptions;
}

/**
 * Toggles an extension between enabled and disabled states
 */
export async function toggleExtension({
  toggle,
  extensionConfig,
  addToConfig,
  toastOptions = {},
}: ToggleExtensionProps) {
  // disabled to enabled
  if (toggle == 'toggleOn') {
    try {
      // add to agent with toast options
      await addToAgent(extensionConfig, {
        ...toastOptions,
      });
    } catch (error) {
      console.error('Error adding extension to agent. Will try to toggle back off.');
      try {
        await toggleExtension({
          toggle: 'toggleOff',
          extensionConfig,
          addToConfig,
          toastOptions: { silent: true }, // otherwise we will see a toast for removing something that was never added
        });
      } catch (toggleError) {
        console.error('Failed to toggle extension off after agent error:', toggleError);
      }
      throw error;
    }

    // update the config
    try {
      await addToConfig(extensionConfig.name, extensionConfig, true);
    } catch (error) {
      console.error('Failed to update config after enabling extension:', error);
      // remove from agent
      try {
        await removeFromAgent(extensionConfig.name, toastOptions);
      } catch (removeError) {
        console.error('Failed to remove extension from agent after config failure:', removeError);
      }
      throw error;
    }
  } else if (toggle == 'toggleOff') {
    // enabled to disabled
    let agentRemoveError = null;
    try {
      await removeFromAgent(extensionConfig.name, toastOptions);
    } catch (error) {
      // note there was an error, but attempt to remove from config anyway
      console.error('Error removing extension from agent', extensionConfig.name, error);
      agentRemoveError = error;
    }

    // update the config
    try {
      await addToConfig(extensionConfig.name, extensionConfig, false);
    } catch (error) {
      console.error('Error removing extension from config', extensionConfig.name, 'Error:', error);
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
}

/**
 * Deletes an extension completely from both agent and config
 */
export async function deleteExtension({ name, removeFromConfig }: DeleteExtensionProps) {
  // remove from agent
  let agentRemoveError = null;
  try {
    await removeFromAgent(name);
  } catch (error) {
    console.error('Failed to remove extension from agent during deletion:', error);
    agentRemoveError = error;
  }

  try {
    await removeFromConfig(name);
  } catch (error) {
    console.error(
      'Failed to remove extension from config after removing from agent. Error:',
      error
    );
    // If we also had an agent remove error, log it but throw the config error as it's more critical
    throw error;
  }

  // If we had an error removing from agent but succeeded removing from config, still throw the original error
  if (agentRemoveError) {
    throw agentRemoveError;
  }
}

export async function saveEnvVarsToKeyring(extension: ExtensionConfig) {
  if (extension.type === 'stdio' || extension.type === 'sse') {
    for (const [key, value] of Object.entries(extension.envs || {})) {
      await upsertConfig({ body: { key, value, is_secret: true } });
    }
  }
}
