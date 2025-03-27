import type { ExtensionConfig } from '../../../api/types.gen';
import { toastService } from '../../../toasts';
import { activateExtension } from './extension-manager';
import { DEFAULT_EXTENSION_TIMEOUT, nameToKey } from './utils';

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
  setView: (view: string, options: { extensionId: string; showEnvVars: boolean }) => void
) {
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

  const cmd = parsedUrl.searchParams.get('cmd');
  if (!cmd) {
    toastService.handleError(
      'Missing Command',
      "Failed to install extension: Missing required 'cmd' parameter in the URL",
      { shouldThrow: true }
    );
  }

  // Validate that the command is one of the allowed commands
  const allowedCommands = ['jbang', 'npx', 'uvx', 'goosed'];
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
  const name = parsedUrl.searchParams.get('name')!;
  const timeout = parsedUrl.searchParams.get('timeout');

  // Create the extension config
  const config: ExtensionConfig = {
    name: name,
    type: 'stdio',
    cmd: cmd,
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
    timeout: timeout ? parseInt(timeout, 10) : DEFAULT_EXTENSION_TIMEOUT,
  };

  // Check if extension requires env vars and go to settings if so
  if (config.envs && Object.keys(config.envs).length > 0) {
    console.log('Environment variables required, redirecting to settings');
    setView('settings', { extensionId: nameToKey(name), showEnvVars: true });
    return;
  }

  // If no env vars are required, proceed with adding the extension
  try {
    await activateExtension({ extensionConfig: config, addToConfig: addExtensionFn });
  } catch (error) {
    console.error('Failed to activate extension from deeplink:', error);
    throw error;
  }
}
