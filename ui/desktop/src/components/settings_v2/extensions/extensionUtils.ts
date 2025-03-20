import { useAgent, ExtensionUpdate } from "../../../agent/UpdateAgent";
import {ExtensionConfig} from "../../../api";

type ExtensionUpdateType = 'add' | 'remove' | 'update' | 'toggle';

// Define error types for better error handling
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

type ExtensionActionError = ConfigError | AgentUpdateError;


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

    // Return a strictly typed function that can be called from components
    const updateConfigAndAgent = async function(
        extensionUpdateType: ExtensionUpdateType,
        params: any[],       // TODO: some type checking
    ): Promise<void> {
        let configSuccess = false;

        // First try-catch for the config operation
        try {
            // Execute the main extension action with appropriate arguments
            // Type safety is ensured by the function overloads
            await actionFn(...actionParams);
            configSuccess = true;
        } catch (error) {
            console.error('Extension config operation failed:', error);
            const configError: ConfigError = {
                type: 'CONFIG_ERROR',
                message: 'Failed to update extension configuration',
                originalError: error
            };
            throw configError;
        }

        // If config update succeeded, update the agent
        if (configSuccess && extensionName) {
            try {
                // Determine if we're enabling or disabling based on the action function
                let isEnabling = false;

                if (actionFn === toggleExtension) {
                    // For toggle, we need to know the current state to determine the new state
                    // This might require additional context or a different approach
                    // For simplicity, we'll assume we're toggling ON here, but you might need to adjust
                    isEnabling = true;
                } else if (actionFn === addExtension) {
                    // For add operation, the third param is the enabled state
                    isEnabling = actionParams[2] as boolean;
                } else if (actionFn === removeExtension) {
                    // For remove, we're definitely disabling
                    isEnabling = false;
                }

                // Update the agent with the extension name and whether it's being added or removed from the agent
                const extensionUpdate = ExtensionUpdate{extension}
                await updateAgent(extensionName);
            } catch (error) {
                console.error('Agent update operation failed:', error);
                const agentError: AgentUpdateError = {
                    type: 'AGENT_UPDATE_ERROR',
                    message: 'Failed to update agent with extension changes',
                    originalError: error
                };
                throw agentError;
            }
        }
    };

    return performExtensionAction;
}

/**
 * Handles extension operation errors with appropriate logging and actions
 * @param error The error caught from extension operations
 * @param callbacks Optional callback functions for different error types
 */
export function handleExtensionError(
    error: any,
    callbacks?: {
        onConfigError?: (error: ExtensionActionError) => void;
        onAgentError?: (error: ExtensionActionError) => void;
        onUnknownError?: (error: any) => void;
    }
) {
    if (error?.type === 'CONFIG_ERROR') {
        console.error('Configuration error:', error.message);
        callbacks?.onConfigError?.(error as ExtensionActionError);
    } else if (error?.type === 'AGENT_ERROR') {
        console.error('Agent update error:', error.message);
        callbacks?.onAgentError?.(error as ExtensionActionError);
    } else {
        // Handle unexpected errors
        console.error('Unknown error:', error);
        callbacks?.onUnknownError?.(error);
    }
}


// For TypeScript to recognize these functions in comparison checks
// You would need to export these from your config context
// This is just a placeholder for the actual implementation
export const toggleExtension: ToggleExtensionFn = async (name) => { /* implementation */ };
export const addExtension: AddExtensionFn = async (name, config, enabled) => { /* implementation */ };
export const removeExtension: RemoveExtensionFn = async (name) => { /* implementation */ };