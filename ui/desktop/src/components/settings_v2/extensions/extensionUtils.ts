import { ExtensionUpdate, useAgent } from '../../../agent/UpdateAgent';
import { ExtensionConfig } from '../../../api';
import { FixedExtensionEntry, useConfig } from '@/src/components/ConfigContext';

type ExtensionUpdateType = 'add' | 'remove' | 'toggle';

interface addExtensionParams {
  name: string;
  extensionConfig: ExtensionConfig;
  enabled: boolean;
}

interface removeExtensionParams {
  name: string;
}

interface toggleExtensionParams {
  name: string;
}

type ExtensionUpdateParams = addExtensionParams | removeExtensionParams | toggleExtensionParams;

// Define error types for better error handling

interface LookupError {
  type: 'LOOKUP_ERROR';
  message: string;
  originalError: unknown;
}

interface ConfigError {
  type: 'CONFIG_ERROR';
  message: string;
  originalError: unknown;
}

interface AgentUpdateError {
  type: 'AGENT_UPDATE_ERROR';
  message: string;
  originalError: unknown;
}

type ExtensionUpdateError = LookupError | ConfigError | AgentUpdateError;

// TODO: make this a backend endpoint we can call that handles the config update part and the agent update part all in one

/**
 * This is a custom hook that handles updates to extensions that should require updates to both the config
 * and the agent.
 *
 * (1) We update the config
 * (2) We update the agent
 *
 * We throw an error if either (1) or (2) fails
 */
export function useExtensionUpdater() {
  const { updateAgent } = useAgent();
  const { addExtension, removeExtension, toggleExtension, getExtensions } = useConfig();

  async function getExtensionEntryFromName(name: string) {
    const extensions = await getExtensions(true);
    const filteredExtensions = extensions.filter((extension) => extension.name === name);
    if (filteredExtensions.length > 1) {
      throw Error(`Multiple extensions with the same name: ${name}`);
    }
    return filteredExtensions[0];
  }

  // Extract just the ExtensionConfig part (omitting the 'enabled' property)
  const extractConfig = (entry: FixedExtensionEntry): ExtensionConfig => {
    const { enabled, ...config } = entry;
    return config;
  };

  // TODO: variable naming
  // Return a strictly typed function that can be called from components
  const updateConfigAndAgent = async function (
    extensionUpdateType: ExtensionUpdateType,
    params: ExtensionUpdateParams
  ): Promise<void> {
    let configSuccess = false;
    let lookupSuccess = false;

    let extensionUpdate = { extension: null, type: null };
    let args = null;
    let extensionToUpdate = null;
    let extEntry = null;

    // get relevant config information
    try {
      switch (extensionUpdateType) {
        case 'add':
          args = params as addExtensionParams;
          extensionUpdate = { extension: args.extensionConfig, type: 'add' };
          break;
        case 'remove':
          args = params as removeExtensionParams;
          extEntry = await getExtensionEntryFromName(args.name);
          extensionToUpdate = extractConfig(extEntry);
          extensionUpdate = { extension: extractConfig(extEntry), type: 'remove' };
          break;
        case 'toggle':
          args = params as toggleExtensionParams;
          extEntry = await getExtensionEntryFromName(params.name);
          extensionUpdate = { extension: extractConfig(extEntry), type: 'add' };
          break;
        default:
          throw Error(
            "trying to perform an operation other other than 'add', 'remove' or 'toggle'"
          );
      }
      lookupSuccess = true;
    } catch (error) {
      const lookupError: LookupError = {
        type: 'LOOKUP_ERROR',
        message: 'Failed to find extension configuration values',
        originalError: error,
      };
      throw lookupError;
    }

    // First try-catch for the config operation
    try {
      switch (extensionUpdateType) {
        case 'add':
          await addExtension(args.name, args.extensionConfig, args.enabled);
          break;
        case 'remove':
          await removeExtension(args.name);
          break;
        case 'toggle':
          await toggleExtension(args.name);
          break;
        default:
          break;
      }
      configSuccess = true;
    } catch (error) {
      console.error('Extension config operation failed:', error);
      const configError: ConfigError = {
        type: 'CONFIG_ERROR',
        message: 'Failed to update extension configuration',
        originalError: error,
      };
      throw configError;
    }

    // If config update succeeded, update the agent
    if (configSuccess && lookupSuccess) {
      try {
        await updateAgent(extensionUpdate);
      } catch (error) {
        console.error('Agent update operation failed:', error);

        // update the config with enabled set to false for this extension
        // TODO: if we tried to remove ext with enabled = true, we should re-try removing ?
        await addExtension(args.name, extensionUpdate.extension, false);

        // TODO: handle retries / config updates with enabled=false with the handleExtensionError callback?

        const agentError: AgentUpdateError = {
          type: 'AGENT_UPDATE_ERROR',
          message: 'Failed to update agent with extension changes',
          originalError: error,
        };
        throw agentError;
      }
    }
  };

  return updateConfigAndAgent;
}

/**
 * Handles extension operation errors with appropriate logging and actions
 * @param error The error caught from extension operations
 * @param callbacks Optional callback functions for different error types
 */
export function handleExtensionError(
  error: any,
  callbacks?: {
    onLookupError?: (error: ExtensionUpdateError) => void;
    onConfigError?: (error: ExtensionUpdateError) => void;
    onAgentError?: (error: ExtensionUpdateError) => void;
    onUnknownError?: (error: any) => void;
  }
) {
  if (error?.type === 'LOOKUP_ERROR') {
    console.error('Lookup error:', error.message);
    callbacks?.onLookupError?.(error as ExtensionUpdateError);
  }
  if (error?.type === 'CONFIG_ERROR') {
    console.error('Configuration error:', error.message);
    callbacks?.onConfigError?.(error as ExtensionUpdateError);
  } else if (error?.type === 'AGENT_ERROR') {
    console.error('Agent update error:', error.message);
    callbacks?.onAgentError?.(error as ExtensionUpdateError);
  } else {
    // Handle unexpected errors
    console.error('Unknown error:', error);
    callbacks?.onUnknownError?.(error);
  }
}
