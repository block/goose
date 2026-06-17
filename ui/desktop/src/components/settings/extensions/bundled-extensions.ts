import type { ExtensionConfig } from '../../../api/types.gen';
import { FixedExtensionEntry } from '../../ConfigContext';
import bundledExtensionsData from './bundled-extensions.json';
import deprecatedBundledExtensionsData from './deprecated-bundled-extensions.json';
import { nameToKey } from './utils';

// Type definition for built-in extensions from JSON
type BundledExtension = {
  id: string;
  name: string;
  display_name?: string;
  description: string;
  enabled: boolean;
  type: 'builtin' | 'stdio' | 'streamable_http';
  cmd?: string;
  args?: string[];
  uri?: string;
  envs?: { [key: string]: string };
  env_keys?: Array<string>;
  timeout?: number;
  allow_configure?: boolean;
};

type DeprecatedBundledExtension = {
  id: string;
};

function arraysEqual(left: string[] = [], right: string[] = []): boolean {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}

function getConfiguredDistroDir(): string | undefined {
  const distroDir = window.appConfig?.get('GOOSE_DISTRO_DIR');
  return typeof distroDir === 'string' && distroDir.trim() ? distroDir.trim() : undefined;
}

function getConfiguredBundledNodeCmd(): string | undefined {
  const nodeCmd = window.appConfig?.get('GOOSE_DESKTOP_STDIO_NODE_CMD');
  return typeof nodeCmd === 'string' && nodeCmd.trim() ? nodeCmd.trim() : undefined;
}

function joinPlatformPath(basePath: string, relativePath: string): string {
  const separator = basePath.includes('\\') ? '\\' : '/';
  const normalizedBase = basePath.replace(/[\\/]+$/, '');
  const normalizedRelative = relativePath.replace(/\//g, separator);
  return `${normalizedBase}${separator}${normalizedRelative}`;
}

function resolveBundledArgs(args: string[] = []): string[] {
  const distroDir = getConfiguredDistroDir();
  if (!distroDir) {
    return args;
  }

  return args.map((arg) => {
    const prefix = 'distro/security-cn/';
    if (!arg.startsWith(prefix)) {
      return arg;
    }

    return joinPlatformPath(distroDir, arg.slice(prefix.length));
  });
}

function isSecurityLocalNodeWrapper(bundledExt: BundledExtension): boolean {
  if (bundledExt.type !== 'stdio' || bundledExt.cmd !== 'node') {
    return false;
  }

  const firstArg = bundledExt.args?.[0];
  return typeof firstArg === 'string' && firstArg.startsWith('distro/security-cn/extensions/');
}

function resolveBundledCommand(bundledExt: BundledExtension): string {
  if (!isSecurityLocalNodeWrapper(bundledExt)) {
    return bundledExt.cmd || '';
  }

  return getConfiguredBundledNodeCmd() || bundledExt.cmd || '';
}

function resolveBundledEnvs(
  bundledExt: BundledExtension
): { [key: string]: string } | undefined {
  if (!isSecurityLocalNodeWrapper(bundledExt)) {
    return bundledExt.envs;
  }

  const nodeCmd = getConfiguredBundledNodeCmd();
  if (!nodeCmd) {
    return bundledExt.envs;
  }

  return {
    ...(bundledExt.envs || {}),
    ELECTRON_RUN_AS_NODE: '1',
  };
}

function createBundledExtensionConfig(bundledExt: BundledExtension): ExtensionConfig {
  switch (bundledExt.type) {
    case 'builtin':
      return {
        type: bundledExt.type,
        name: bundledExt.name,
        description: bundledExt.description,
        display_name: bundledExt.display_name,
        timeout: bundledExt.timeout ?? 300,
        bundled: true,
      };
    case 'stdio':
      return {
        type: bundledExt.type,
        name: bundledExt.name,
        description: bundledExt.description,
        timeout: bundledExt.timeout,
        cmd: resolveBundledCommand(bundledExt),
        args: resolveBundledArgs(bundledExt.args || []),
        envs: resolveBundledEnvs(bundledExt),
        env_keys: bundledExt.env_keys || [],
        bundled: true,
      };
    case 'streamable_http':
      return {
        type: bundledExt.type,
        name: bundledExt.name,
        description: bundledExt.description,
        timeout: bundledExt.timeout,
        uri: bundledExt.uri || '',
        bundled: true,
      };
  }
}

function matchesBundledConfig(
  existingExt: FixedExtensionEntry | undefined,
  expectedConfig: ExtensionConfig
): boolean {
  if (!existingExt) {
    return false;
  }

  if (existingExt.type !== expectedConfig.type || existingExt.name !== expectedConfig.name) {
    return false;
  }

  if (existingExt.description !== expectedConfig.description) {
    return false;
  }

  if (expectedConfig.type === 'builtin' && existingExt.type === 'builtin') {
    return (
      existingExt.display_name === expectedConfig.display_name &&
      (existingExt.timeout ?? 300) === (expectedConfig.timeout ?? 300)
    );
  }

  if (expectedConfig.type === 'stdio' && existingExt.type === 'stdio') {
    return (
      existingExt.cmd === expectedConfig.cmd &&
      arraysEqual(existingExt.args, expectedConfig.args) &&
      arraysEqual(existingExt.env_keys, expectedConfig.env_keys) &&
      (existingExt.timeout ?? undefined) === (expectedConfig.timeout ?? undefined)
    );
  }

  if (expectedConfig.type === 'streamable_http' && existingExt.type === 'streamable_http') {
    return (
      existingExt.uri === expectedConfig.uri &&
      (existingExt.timeout ?? undefined) === (expectedConfig.timeout ?? undefined)
    );
  }

  return false;
}

export function getDeprecatedBundledExtensions(): DeprecatedBundledExtension[] {
  return deprecatedBundledExtensionsData as DeprecatedBundledExtension[];
}

function isBundledExtension(extension: FixedExtensionEntry): boolean {
  return 'bundled' in extension && extension.bundled === true;
}

export async function pruneDeprecatedBundledExtensions(
  existingExtensions: FixedExtensionEntry[],
  removeExtensionFn: (id: string) => Promise<void>
): Promise<FixedExtensionEntry[]> {
  const deprecatedExtensionIds = new Set(getDeprecatedBundledExtensions().map((ext) => ext.id));
  const remainingExtensions: FixedExtensionEntry[] = [];

  for (const existingExt of existingExtensions) {
    if (!isBundledExtension(existingExt)) {
      remainingExtensions.push(existingExt);
      continue;
    }

    if (!deprecatedExtensionIds.has(nameToKey(existingExt.name))) {
      remainingExtensions.push(existingExt);
      continue;
    }

    await removeExtensionFn(nameToKey(existingExt.name));
  }

  return remainingExtensions;
}

/**
 * Synchronizes built-in extensions with the config system.
 * This function ensures all built-in extensions are added, which is especially
 * important for first-time users with an empty config.yaml.
 *
 * @param existingExtensions Current list of extensions from the config (could be empty)
 * @param addExtensionFn Function to add a new extension to the config
 * @returns Promise that resolves when sync is complete
 */
export async function syncBundledExtensions(
  existingExtensions: FixedExtensionEntry[],
  addExtensionFn: (name: string, config: ExtensionConfig, enabled: boolean) => Promise<void>
): Promise<void> {
  try {
    // Cast the imported JSON data to the expected type
    const bundledExtensions = bundledExtensionsData as BundledExtension[];

    // Process each bundled extension
    for (const bundledExt of bundledExtensions) {
      // Find if this extension already exists
      const existingExt = existingExtensions.find((ext) => nameToKey(ext.name) === bundledExt.id);
      const extConfig = createBundledExtensionConfig(bundledExt);

      if (existingExt && isBundledExtension(existingExt) && matchesBundledConfig(existingExt, extConfig)) {
        continue;
      }

      // Add or update the extension, preserving enabled state if it exists
      const enabled = existingExt ? existingExt.enabled : bundledExt.enabled;
      await addExtensionFn(bundledExt.name, extConfig, enabled);
    }
  } catch (error) {
    console.error('Failed to sync built-in extensions:', error);
    throw error;
  }
}
