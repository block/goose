import { authenticateConfigExtension } from '../../../acp/extensions';
import { toastService } from '../../../toasts';
import { errorMessage } from '../../../utils/conversionUtils';
import { nameToKey } from './utils';

/** Custom URL scheme registered for Avocado Work (see forge.config.ts). */
export const APP_DEEPLINK_SCHEME = 'avocado-work';

/** Legacy goose scheme still used in dev builds and upstream docs. */
export const LEGACY_DEEPLINK_SCHEME = 'goose';

const SUPPORTED_SCHEMES = new Set([APP_DEEPLINK_SCHEME, LEGACY_DEEPLINK_SCHEME]);

export function buildExtensionAuthenticateLink(
  configKey: string,
  options?: { force?: boolean; scheme?: string }
): string {
  const scheme = options?.scheme ?? APP_DEEPLINK_SCHEME;
  const url = new URL(`${scheme}://extension-authenticate`);
  url.searchParams.set('configKey', configKey);
  if (options?.force) {
    url.searchParams.set('force', 'true');
  }
  return url.toString();
}

export function isExtensionAuthenticateDeepLink(url: string): boolean {
  try {
    const parsed = new URL(url);
    if (!SUPPORTED_SCHEMES.has(parsed.protocol.replace(':', ''))) {
      return false;
    }
    if (parsed.hostname === 'extension-authenticate') {
      return true;
    }
    return (
      parsed.hostname === 'extension' && parsed.searchParams.get('action') === 'authenticate'
    );
  } catch {
    return false;
  }
}

export function parseExtensionAuthenticateDeepLink(url: string): {
  configKey: string;
  force: boolean;
} {
  const parsed = new URL(url);
  if (!SUPPORTED_SCHEMES.has(parsed.protocol.replace(':', ''))) {
    throw new Error('Invalid protocol: URL must use avocado-work:// or goose://');
  }

  const isAuthHost =
    parsed.hostname === 'extension-authenticate' ||
    (parsed.hostname === 'extension' && parsed.searchParams.get('action') === 'authenticate');

  if (!isAuthHost) {
    throw new Error('Not an extension authentication link');
  }

  const configKey =
    parsed.searchParams.get('configKey')?.trim() ||
    parsed.searchParams.get('id')?.trim() ||
    (parsed.searchParams.get('name') ? nameToKey(parsed.searchParams.get('name')!) : '');

  if (!configKey) {
    throw new Error('Authentication link is missing configKey (or id / name)');
  }

  const forceParam = parsed.searchParams.get('force');
  const force = forceParam === 'true' || forceParam === '1';

  return { configKey, force };
}

export async function authenticateExtensionFromDeepLink(url: string): Promise<void> {
  const { configKey, force } = parseExtensionAuthenticateDeepLink(url);

  try {
    await authenticateConfigExtension(configKey, { force });
    toastService.success({
      title: configKey,
      msg: 'Signed in successfully. New chats can use this connection.',
    });
  } catch (error) {
    toastService.error({
      title: configKey,
      msg: 'Sign in failed',
      traceback: errorMessage(error),
    });
    throw error;
  }
}
