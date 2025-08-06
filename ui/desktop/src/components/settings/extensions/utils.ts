// Default extension timeout in seconds
// TODO: keep in sync with rust better

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
  type: 'stdio' | 'sse' | 'streamable_http' | 'builtin';
  cmd?: string;
  endpoint?: string;
  enabled: boolean;
  timeout?: number;
  envVars: {
    key: string;
    value: string;
    isEdited?: boolean;
  }[];
  headers: {
    key: string;
    value: string;
    isEdited?: boolean;
  }[];
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
    headers: [],
  };
}

export function extensionToFormData(extension: FixedExtensionEntry): ExtensionFormData {
  // Type guard: Check if 'envs' property exists for this variant
  const hasEnvs =
    extension.type === 'sse' || extension.type === 'streamable_http' || extension.type === 'stdio';

  // Handle both envs (legacy) and env_keys (new secrets)
  let envVars = [];

  // Add legacy envs with their values
  if (hasEnvs && extension.envs) {
    envVars.push(
      ...Object.entries(extension.envs).map(([key, value]) => ({
        key,
        value: value as string,
        isEdited: true, // We want to submit legacy values as secrets to migrate forward
      }))
    );
  }

  // Add env_keys with placeholder values
  if (hasEnvs && extension.env_keys) {
    envVars.push(
      ...extension.env_keys.map((key) => ({
        key,
        value: '••••••••', // Placeholder for secret values
        isEdited: false, // Mark as not edited initially
      }))
    );
  }

  // Handle headers for streamable_http
  let headers = [];
  if (extension.type === 'streamable_http' && 'headers' in extension && extension.headers) {
    headers.push(
      ...Object.entries(extension.headers).map(([key, value]) => ({
        key,
        value: value as string,
        isEdited: false, // Mark as not edited initially
      }))
    );
  }

  return {
    name: extension.name || '',
    description:
      extension.type === 'stdio' || extension.type === 'sse' || extension.type === 'streamable_http'
        ? extension.description || ''
        : '',
    type:
      extension.type === 'frontend' || extension.type === 'inline_python'
        ? 'stdio'
        : extension.type,
    cmd: extension.type === 'stdio' ? combineCmdAndArgs(extension.cmd, extension.args) : undefined,
    endpoint:
      extension.type === 'sse' || extension.type === 'streamable_http' ? extension.uri : undefined,
    enabled: extension.enabled,
    timeout: 'timeout' in extension ? (extension.timeout ?? undefined) : undefined,
    envVars,
    headers,
  };
}

export function createExtensionConfig(formData: ExtensionFormData): ExtensionConfig {
  // Extract just the keys from env vars
  const env_keys = formData.envVars.map(({ key }) => key).filter((key) => key.length > 0);

  if (formData.type === 'stdio') {
    // we put the cmd + args all in the form cmd field but need to split out into cmd + args
    const { cmd, args } = splitCmdAndArgs(formData.cmd || '');

    return {
      type: 'stdio',
      name: formData.name,
      description: formData.description,
      cmd: cmd,
      args: args,
      timeout: formData.timeout,
      ...(env_keys.length > 0 ? { env_keys } : {}),
    };
  } else if (formData.type === 'sse') {
    return {
      type: 'sse',
      name: formData.name,
      description: formData.description,
      timeout: formData.timeout,
      uri: formData.endpoint || '',
      ...(env_keys.length > 0 ? { env_keys } : {}),
    };
  } else if (formData.type === 'streamable_http') {
    // Extract headers
    const headers = formData.headers
      .filter(({ key, value }) => key.length > 0 && value.length > 0)
      .reduce(
        (acc, header) => {
          acc[header.key] = header.value;
          return acc;
        },
        {} as Record<string, string>
      );

    return {
      type: 'streamable_http',
      name: formData.name,
      description: formData.description,
      timeout: formData.timeout,
      uri: formData.endpoint || '',
      ...(env_keys.length > 0 ? { env_keys } : {}),
      ...(Object.keys(headers).length > 0 ? { headers } : {}),
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
  // todo: enabled not used?
  const { ...extensionConfig } = fixedEntry;
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
  // Only remove shims if the path matches our known shim patterns
  const shimPatterns = [/cu$/, /goosed$/, /docker$/, /jbang$/, /npx$/, /uvx$/, /npx.cmd$/];

  // Check if the command matches any shim pattern
  const isShim = shimPatterns.some((pattern) => pattern.test(cmd));

  if (isShim) {
    const segments = cmd.split('/');
    // Filter out any empty segments (which can happen with trailing slashes)
    const nonEmptySegments = segments.filter((segment) => segment.length > 0);
    // Return the last segment or empty string if there are no segments
    return nonEmptySegments.length > 0 ? nonEmptySegments[nonEmptySegments.length - 1] : '';
  }

  // If it's not a shim, return the original command
  return cmd;
}

export function extractCommand(link: string): string {
  const url = new URL(link);
  const cmd = url.searchParams.get('cmd') || 'Unknown Command';
  const args = url.searchParams.getAll('arg').map(decodeURIComponent);

  // Combine the command and its arguments into a reviewable format
  return `${cmd} ${args.join(' ')}`.trim();
}

export function extractExtensionName(link: string): string {
  const url = new URL(link);
  const name = url.searchParams.get('name');
  return name ? decodeURIComponent(name) : 'Unknown Extension';
}
