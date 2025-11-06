/**
 * Shared constants and utilities for extension error handling
 */

export const MAX_ERROR_MESSAGE_LENGTH = 70;

/**
 * Creates recovery hints for the "Ask goose" feature when extension loading fails
 */
export function createExtensionRecoverHints(errorMsg: string): string {
  return (
    `Explain the following error: ${errorMsg}. ` +
    'This happened while trying to install an extension. Look out for issues where the ' +
    "extension attempted to execute something incorrectly, didn't exist, or there was trouble with " +
    'the network configuration - VPNs like WARP often cause issues.'
  );
}

/**
 * Formats an error message for display, truncating long messages with a fallback
 * @param errorMsg - The full error message
 * @param fallback - The fallback message to show if the error is too long
 * @returns The formatted error message
 */
export function formatExtensionErrorMessage(
  errorMsg: string,
  fallback: string = 'Failed to add extension'
): string {
  return errorMsg.length < MAX_ERROR_MESSAGE_LENGTH ? errorMsg : fallback;
}
