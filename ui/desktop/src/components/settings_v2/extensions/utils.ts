// Default extension timeout in seconds
// TODO: keep in sync with rust better
import * as module from 'node:module';

export const DEFAULT_EXTENSION_TIMEOUT = 300;

/**
 * Converts an extension name to a key format
 * TODO: need to keep this in sync better with `name_to_key` on the rust side
 */
export function nameToKey(name: string): string {
  return name
    .split('')
    .filter((char) => !char.match(/\s/))
    .join('')
    .toLowerCase();
}

import { FixedExtensionEntry } from '../../ConfigContext';
import { ExtensionConfig } from '../../../api/types.gen';

export interface ExtensionFormData {
  name: string;
  description: string;
  type: 'stdio' | 'sse' | 'builtin';
  cmd?: string;
  endpoint?: string;
  enabled: boolean;
  timeout?: number;
  envVars: { key: string; value: string }[];
}

export function getDefaultFormData(): ExtensionFormData {
  return {
    name: '',
    description: '',
    type: 'stdio',
    cmd: '',
    endpoint: '',
    enabled: true,
    timeout: 300,
    envVars: [],
  };
}

export function extensionToFormData(extension: FixedExtensionEntry): ExtensionFormData {
  // Type guard: Check if 'envs' property exists for this variant
  const hasEnvs = extension.type === 'sse' || extension.type === 'stdio';

  const envVars =
    hasEnvs && extension.envs
      ? Object.entries(extension.envs).map(([key, value]) => ({
          key,
          value: value as string,
        }))
      : [];

  return {
    name: extension.name,
    description:
      extension.type === 'stdio' || extension.type === 'sse' ? extension.description : undefined,
    type: extension.type,
    cmd: extension.type === 'stdio' ? combineCmdAndArgs(extension.cmd, extension.args) : undefined,
    endpoint: extension.type === 'sse' ? extension.uri : undefined,
    enabled: extension.enabled,
    timeout: extension.timeout,
    envVars,
  };
}

export function createExtensionConfig(formData: ExtensionFormData): ExtensionConfig {
  const envs = formData.envVars.reduce(
    (acc, { key, value }) => {
      if (key) {
        acc[key] = value;
      }
      return acc;
    },
    {} as Record<string, string>
  );

  if (formData.type === 'stdio') {
    // we put the cmd + args all in the form cmd field but need to split out into cmd + args
    const { cmd, args } = splitCmdAndArgs(formData.cmd);

    return {
      type: 'stdio',
      name: formData.name,
      description: formData.description,
      cmd: cmd,
      args: args,
      timeout: formData.timeout,
      ...(Object.keys(envs).length > 0 ? { envs } : {}),
    };
  } else if (formData.type === 'sse') {
    return {
      type: 'sse',
      name: formData.name,
      description: formData.description,
      timeout: formData.timeout,
      uri: formData.endpoint, // Assuming endpoint maps to uri for SSE type
      ...(Object.keys(envs).length > 0 ? { envs } : {}),
    };
  } else {
    // For other types
    return {
      type: formData.type,
      name: formData.name,
      timeout: formData.timeout,
    };
  }
}

export function splitCmdAndArgs(str: string): { cmd: string; args: string[] } {
  const words = str.trim().split(/\s+/);
  const cmd = words[0] || '';
  const args = words.slice(1);

  return {
    cmd,
    args,
  };
}

export function combineCmdAndArgs(cmd: string, args: string[]): string {
  return [cmd, ...args].join(' ');
}

/**
 * Extracts the ExtensionConfig from a FixedExtensionEntry object
 * @param fixedEntry - The FixedExtensionEntry object
 * @returns The ExtensionConfig portion of the object
 */
export function extractExtensionConfig(fixedEntry: FixedExtensionEntry): ExtensionConfig {
  const { enabled, ...extensionConfig } = fixedEntry;
  return extensionConfig;
}

export async function replaceWithShims(cmd: string) {
  const binaryPathMap: Record<string, string> = {
    goosed: await window.electron.getBinaryPath('goosed'),
    jbang: await window.electron.getBinaryPath('jbang'),
    npx: await window.electron.getBinaryPath('npx'),
    uvx: await window.electron.getBinaryPath('uvx'),
  };

  if (binaryPathMap[cmd]) {
    console.log('--------> Replacing command with shim ------>', cmd, binaryPathMap[cmd]);
    cmd = binaryPathMap[cmd];
  }

  return cmd;
}

export function removeShims(cmd: string) {
  const segments = cmd.split('/');
  // Filter out any empty segments (which can happen with trailing slashes)
  const nonEmptySegments = segments.filter((segment) => segment.length > 0);
  // Return the last segment or empty string if there are no segments
  return nonEmptySegments.length > 0 ? nonEmptySegments[nonEmptySegments.length - 1] : '';
}
