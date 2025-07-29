import type { ExtensionConfig } from '../../../api';
import { toastService } from '../../../toasts';
import { activateExtension } from './extension-manager';
import { DEFAULT_EXTENSION_TIMEOUT } from './utils';

/**
 * Build an extension config for stdio from the deeplink URL
 */
function getStdioConfig(
  cmd: string,
  parsedUrl: URL,
  name: string,
  description: string,
  timeout: number
) {
  // Validate that the command is one of the allowed commands
  const allowedCommands = ['cu', 'docker', 'jbang', 'npx', 'uvx', 'goosed', 'npx.cmd'];
  if (!allowedCommands.includes(cmd)) {
    toastService.handleError(
      'Invalid Command',
      `Failed to install extension: Invalid command: ${cmd}. Only ${allowedCommands.join(', ')} are allowed.`,
      { shouldThrow: true }
    );
  }

  // Check for security risk with npx -c command
  const args = parsedUrl.searchParams.getAll('arg');
  if (cmd === 'npx' && args.includes('-c')) {
    toastService.handleError(
      'Security Risk',
      'Failed to install extension: npx with -c argument can lead to code injection',
      { shouldThrow: true }
    );
  }

  const envList = parsedUrl.searchParams.getAll('env');

  // Create the extension config
  const config: ExtensionConfig = {
    name: name,
    type: 'stdio',
    cmd: cmd,
    description,
    args: args,
    envs:
      envList.length > 0
        ? Object.fromEntries(
            envList.map((env) => {
              const [key] = env.split('=');
              return [key, '']; // Initialize with empty string as value
            })
          )
        : undefined,
    timeout: timeout,
  };

  return config;
}

/**
 * Build an extension config for SSE from the deeplink URL
 */
function getSseConfig(remoteUrl: string, name: string, description: string, timeout: number) {
  const config: ExtensionConfig = {
    name,
    type: 'sse',
    uri: remoteUrl,
    description,
    timeout: timeout,
  };

  return config;
}

/**
 * Build an extension config for Streamable HTTP from the deeplink URL
 */
function getStreamableHttpConfig(
  remoteUrl: string,
  name: string,
  description: string,
  timeout: number
) {
  const config: ExtensionConfig = {
    name,
    type: 'streamable_http',
    uri: remoteUrl,
    description,
    timeout: timeout,
  };

  return config;
}

/**
 * Handles adding an extension from a deeplink URL
 */
export async function addExtensionFromDeepLink(
  url: string,
  addExtensionFn: (
    name: string,
    extensionConfig: ExtensionConfig,
    enabled: boolean
  ) => Promise<void>,
  setView: (
    view: string,
    options:
      | { extensionId: string; showEnvVars: boolean }
      | { deepLinkConfig: ExtensionConfig; showEnvVars: boolean }
  ) => void
) {
  console.log('=== addExtensionFromDeepLink Debug ===');
  console.log('URL:', url);

  const parsedUrl = new URL(url);

  if (parsedUrl.protocol !== 'goose:') {
    toastService.handleError(
      'Invalid Protocol',
      'Failed to install extension: Invalid protocol: URL must use the goose:// scheme',
      { shouldThrow: true }
    );
  }

  // Check that all required fields are present and not empty
  const requiredFields = ['name'];

  for (const field of requiredFields) {
    const value = parsedUrl.searchParams.get(field);
    if (!value || value.trim() === '') {
      toastService.handleError(
        'Missing Field',
        `Failed to install extension: The link is missing required field '${field}'`,
        { shouldThrow: true }
      );
    }
  }

  const name = parsedUrl.searchParams.get('name')!;
  const parsedTimeout = parsedUrl.searchParams.get('timeout');
  const timeout = parsedTimeout ? parseInt(parsedTimeout, 10) : DEFAULT_EXTENSION_TIMEOUT;
  const description = parsedUrl.searchParams.get('description');

  const cmd = parsedUrl.searchParams.get('cmd');
  const remoteUrl = parsedUrl.searchParams.get('url');
  const transportType = parsedUrl.searchParams.get('transport') || 'sse'; // Default to SSE for backward compatibility

  const config = remoteUrl
    ? transportType === 'streamable_http'
      ? getStreamableHttpConfig(remoteUrl, name, description || '', timeout)
      : getSseConfig(remoteUrl, name, description || '', timeout)
    : getStdioConfig(cmd!, parsedUrl, name, description || '', timeout);

  // Check if extension requires env vars and go to settings if so
  if (config.envs && Object.keys(config.envs).length > 0) {
    console.log('Environment variables required, redirecting to settings');
    console.log('Calling setView with:', { deepLinkConfig: config, showEnvVars: true });
    setView('settings', { deepLinkConfig: config, showEnvVars: true });
    return;
  }

  // If no env vars are required, proceed with adding the extension
  try {
    console.log('No env vars required, activating extension directly');
    await activateExtension({ extensionConfig: config, addToConfig: addExtensionFn });
  } catch (error) {
    console.error('Failed to activate extension from deeplink:', error);
    throw error;
  }
}
