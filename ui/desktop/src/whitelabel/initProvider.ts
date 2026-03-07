/**
 * Auto-configure provider, model, and extensions from whitelabel config.
 *
 * Called from the Electron main process after goosed is ready.
 * - Registers the custom provider definition if declared
 * - Sets GOOSE_PROVIDER and GOOSE_MODEL so the agent knows what to use
 * - Configures extensions (enable/disable per whitelabel spec)
 *
 * Idempotent — safe to call on every launch.
 */

import * as fs from 'node:fs';
import * as path from 'node:path';
import type { Client } from '../api/client';
import {
  getCustomProvider,
  createCustomProvider,
  upsertConfig,
  readConfig,
  getExtensions,
  addExtension,
} from '../api';
import type { WhiteLabelConfig } from './types';
import type { ExtensionConfig, ExtensionEntry } from '../api';
import log from '../utils/logger';

function extractConfig(entry: ExtensionEntry): ExtensionConfig {
  const { enabled: _, ...config } = entry;
  return config as ExtensionConfig;
}

export async function initWhiteLabelProvider(
  client: Client,
  config: WhiteLabelConfig,
  workingDir?: string
): Promise<void> {
  const { defaults } = config;

  // 1. Register custom provider if definition is provided
  if (defaults.providerDefinition) {
    await registerProvider(client, defaults);
  }

  // 2. Set GOOSE_PROVIDER
  if (defaults.provider) {
    await setConfigIfChanged(client, 'GOOSE_PROVIDER', defaults.provider);
  }

  // 3. Set GOOSE_MODEL
  if (defaults.model) {
    await setConfigIfChanged(client, 'GOOSE_MODEL', defaults.model);
  }

  // 4. Configure extensions
  if (defaults.extensions && defaults.extensions.length > 0) {
    await configureExtensions(client, defaults.extensions);
  }

  // 5. Write .goosehints if configured and not already present
  if (defaults.goosehints && workingDir) {
    const hintsPath = path.join(workingDir, '.goosehints');
    if (!fs.existsSync(hintsPath)) {
      try {
        fs.writeFileSync(hintsPath, defaults.goosehints, 'utf-8');
        log.info(`[whitelabel] Wrote .goosehints to ${hintsPath}`);
      } catch (err) {
        log.warn(`[whitelabel] Failed to write .goosehints:`, err);
      }
    }
  }
}

async function registerProvider(client: Client, defaults: WhiteLabelConfig['defaults']) {
  const providerDef = defaults.providerDefinition!;
  try {
    const existing = await getCustomProvider({
      client,
      path: { id: providerDef.id },
      throwOnError: false,
    });

    if (existing.error) {
      const res = await createCustomProvider({
        client,
        body: {
          display_name: providerDef.displayName,
          engine: providerDef.engine,
          api_url: providerDef.apiUrl,
          models: providerDef.models,
          requires_auth: providerDef.requiresAuth ?? false,
          supports_streaming: providerDef.supportsStreaming ?? false,
          headers: providerDef.headers ?? null,
          base_path: providerDef.basePath ?? null,
          api_key: '',
        },
        throwOnError: false,
      });

      if (res.error) {
        log.warn(`[whitelabel] Failed to create provider ${providerDef.id}:`, res.error);
      } else {
        log.info(`[whitelabel] Registered custom provider: ${providerDef.id}`);
      }
    } else {
      log.info(`[whitelabel] Provider ${providerDef.id} already registered`);
    }
  } catch (err) {
    log.warn(`[whitelabel] Failed to register provider ${providerDef.id}:`, err);
  }
}

async function setConfigIfChanged(client: Client, key: string, value: string) {
  try {
    const currentRes = await readConfig({
      client,
      body: { key, is_secret: false },
      throwOnError: false,
    });
    const current = currentRes.data as string | undefined;

    if (current !== value) {
      await upsertConfig({
        client,
        body: { key, value, is_secret: false },
        throwOnError: true,
      });
      log.info(`[whitelabel] Set ${key} = ${value}`);
    }
  } catch (err) {
    log.warn(`[whitelabel] Failed to set ${key}:`, err);
  }
}

async function configureExtensions(
  client: Client,
  extensionDefaults: WhiteLabelConfig['defaults']['extensions']
) {
  if (!extensionDefaults || extensionDefaults.length === 0) return;

  try {
    // Build a map of desired states from whitelabel config
    const desired = new Map(extensionDefaults.map((e) => [e.name, e]));

    // Get current extensions from goosed
    const currentRes = await getExtensions({ client, throwOnError: false });
    const currentExtensions = currentRes.data?.extensions ?? [];
    const currentMap = new Map(currentExtensions.map((e) => [e.name, e]));

    for (const ext of extensionDefaults) {
      const current = currentMap.get(ext.name);
      const needsUpdate = !current || current.enabled !== ext.enabled;

      if (needsUpdate) {
        // Build the ExtensionConfig based on type
        let extConfig: Record<string, unknown>;
        if (ext.type === 'builtin') {
          extConfig = {
            type: 'builtin',
            name: ext.name,
            description: '',
          };
        } else if (ext.type === 'platform') {
          extConfig = {
            type: 'platform',
            name: ext.name,
            description: '',
          };
        } else if (ext.type === 'sse' && ext.uri) {
          extConfig = {
            type: 'sse',
            name: ext.name,
            description: '',
            uri: ext.uri,
          };
        } else if (ext.type === 'stdio' && ext.cmd) {
          extConfig = {
            type: 'stdio',
            name: ext.name,
            description: '',
            cmd: ext.cmd,
            args: ext.args ?? [],
            ...(ext.envVars ? { envs: ext.envVars } : {}),
          };
        } else {
          // Use existing config if available, just toggle enabled
          if (current) {
            extConfig = extractConfig(current) as unknown as Record<string, unknown>;
          } else {
            log.warn(`[whitelabel] Unknown extension type for ${ext.name}: ${ext.type}`);
            continue;
          }
        }

        await addExtension({
          client,
          body: {
            name: ext.name,
            enabled: ext.enabled,
            config: extConfig as ExtensionConfig,
          },
          throwOnError: false,
        });

        log.info(`[whitelabel] Extension ${ext.name}: enabled=${ext.enabled}`);
      }
    }

    // Disable any extensions not in the whitelabel config
    for (const [name, entry] of currentMap) {
      if (!desired.has(name) && entry.enabled) {
        await addExtension({
          client,
          body: {
            name,
            enabled: false,
            config: extractConfig(entry) as ExtensionConfig,
          },
          throwOnError: false,
        });
        log.info(`[whitelabel] Extension ${name}: disabled (not in whitelabel config)`);
      }
    }
  } catch (err) {
    log.warn('[whitelabel] Failed to configure extensions:', err);
  }
}
