import { importSessionNostr } from './api';

/**
 * Imports a session from an encrypted Nostr deep link.
 */
export async function importNostrSessionFromDeepLink(url: string): Promise<void> {
  await importSessionNostr({
    body: { deeplink: url },
    throwOnError: true,
  });
}
