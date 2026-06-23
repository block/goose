import type { ExtensionConfig } from '../api';
import { getAcpClient } from './acpConnection';
import { extensionConfigToGooseExtension, gooseExtensionToExtensionConfig } from './extensions';

export async function getSessionExtensions(sessionId: string): Promise<ExtensionConfig[]> {
  const client = await getAcpClient();
  const response = await client.goose.sessionExtensionsList_unstable({ sessionId });
  return response.extensions
    .map(gooseExtensionToExtensionConfig)
    .filter((config): config is ExtensionConfig => config !== null);
}

export async function addSessionExtension(
  sessionId: string,
  config: ExtensionConfig
): Promise<void> {
  const extension = extensionConfigToGooseExtension(config);
  if (!extension) {
    throw new Error(`Unsupported extension type for ACP: ${config.type}`);
  }
  const client = await getAcpClient();
  await client.goose.sessionExtensionsAdd_unstable({ sessionId, extension });
}

export async function removeSessionExtension(sessionId: string, name: string): Promise<void> {
  const client = await getAcpClient();
  await client.goose.sessionExtensionsRemove_unstable({ sessionId, name });
}
