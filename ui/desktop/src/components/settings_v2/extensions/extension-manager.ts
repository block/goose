import type { ExtensionConfig } from '../../../api/types.gen';
import { ToastServiceOptions } from '../../../toasts';
import { addToAgent, removeFromAgent } from './agent-api';

interface ActivateExtensionProps {
  addToConfig: (name: string, extensionConfig: ExtensionConfig, enabled: boolean) => Promise<void>;
  extensionConfig: ExtensionConfig;
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
    await addToAgent(extensionConfig);
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
  const MAX_RETRIES = 3;
  const RETRY_DELAY = 1000; // 1 second delay between retries

  let retries = 0;

  while (retries <= MAX_RETRIES) {
    try {
      // Use silent mode for startup
      await addToAgent(extensionConfig, { silent: true, showEscMessage: false });
      // If successful, break out of the retry loop
      break;
    } catch (error) {
      console.log(`Attempt ${retries + 1} failed when adding extension to agent:`, error);

      // Check if this is a 428 error (agent not initialized)
      const is428Error =
        error.message &&
        (error.message.includes('428') ||
          error.message.includes('Precondition Required') ||
          error.message.includes('Agent is not initialized'));

      // retry adding a few times if agent is spinning up
      if (is428Error && retries < MAX_RETRIES) {
        // This is a 428 error and we have retries left
        retries++;
        console.log(
          `Agent not initialized yet. Retrying in ${RETRY_DELAY}ms... (${retries}/${MAX_RETRIES})`
        );
        // Wait before retrying
        await new Promise((resolve) => setTimeout(resolve, RETRY_DELAY));
        continue;
      }

      // Either not a 428 error or we've exhausted retries
      console.error('Failed to add to agent after retries or due to other error:', error);

      // update config with enabled = false because we weren't able to install the extension
      try {
        await toggleExtension({
          toggle: 'toggleOff',
          extensionConfig,
          addToConfig,
          toastOptions: { silent: true }, // on startup, let extensions fail silently
        });
      } catch (toggleError) {
        console.error('Failed to toggle extension off after agent error:', toggleError);
      }

      // Rethrow the error to inform the caller
      throw error;
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
        // For toggle operations, we want to show toast but no ESC message
        showEscMessage: false,
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
