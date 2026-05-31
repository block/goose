import type { ExtensionEntry, ExtensionConfig } from '../api';
import { getAcpClient } from './acpConnection';

export interface ConfiguredExtensionsResponse {
  extensions: ExtensionEntry[];
  warnings: string[];
}

/**
 * Fetch all configured extensions via ACP (`_goose/unstable/config/extensions/list`).
 */
export async function getConfiguredExtensions(): Promise<ConfiguredExtensionsResponse> {
  const client = await getAcpClient();
  const response = await client.goose.configExtensionsList_unstable({});
  return {
    extensions: response.extensions as ExtensionEntry[],
    warnings: response.warnings ?? [],
  };
}

/**
 * Add (or update) an extension in the user's global goose config via ACP
 * (`_goose/unstable/config/extensions/add`).
 */
export async function addConfiguredExtension(
  name: string,
  config: ExtensionConfig,
  enabled: boolean
): Promise<void> {
  const client = await getAcpClient();
  // Server expects a JSON object matching one of the ExtensionConfig variants,
  // and injects `name` itself. We strip `name` from the body to match that shape.
  const extensionConfig = { ...config } as Record<string, unknown>;
  delete extensionConfig.name;

  await client.goose.configExtensionsAdd_unstable({
    name,
    extensionConfig,
    enabled,
  });
}

/**
 * Remove an extension from the user's global goose config via ACP
 * (`_goose/unstable/config/extensions/remove`). The server normalizes the
 * supplied `configKey` via `name_to_key`, so passing the raw extension name
 * is sufficient and matches how the previous REST route worked.
 */
export async function removeConfiguredExtension(name: string): Promise<void> {
  const client = await getAcpClient();
  await client.goose.configExtensionsRemove_unstable({
    configKey: name,
  });
}

/**
 * Add an extension to a running session's agent via ACP
 * (`_goose/unstable/session/extensions/add`).
 */
export async function addSessionExtension(
  sessionId: string,
  config: ExtensionConfig
): Promise<void> {
  const client = await getAcpClient();
  await client.goose.sessionExtensionsAdd_unstable({
    sessionId,
    config,
  });
}

/**
 * Remove an extension from a running session's agent via ACP
 * (`_goose/unstable/session/extensions/remove`).
 */
export async function removeSessionExtension(
  sessionId: string,
  name: string
): Promise<void> {
  const client = await getAcpClient();
  await client.goose.sessionExtensionsRemove_unstable({
    sessionId,
    name,
  });
}

/**
 * Fetch the list of extensions associated with a given session via ACP
 * (`_goose/unstable/session/extensions/list`).
 */
export async function getSessionExtensions(
  sessionId: string
): Promise<ExtensionEntry[]> {
  const client = await getAcpClient();
  const response = await client.goose.sessionExtensionsList_unstable({ sessionId });
  return response.extensions as ExtensionEntry[];
}
