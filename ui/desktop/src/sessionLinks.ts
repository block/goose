import { acpImportSession } from './acp/sessions';

/**
 * Imports a session from an encrypted Nostr deep link.
 */
export async function importNostrSessionFromDeepLink(url: string): Promise<void> {
  await acpImportSession(url, 'nostr');
}
