import { FixedExtensionEntry } from '../../ConfigContext';
import { ExtensionConfig } from '../../../api/types.gen';

export interface ExtensionFormData {
  name: string;
  type: 'stdio' | 'sse' | 'builtin';
  cmd?: string;
  endpoint?: string;
  enabled: boolean;
  envVars: { key: string; value: string }[];
}

export function getDefaultFormData(): ExtensionFormData {
  return {
    name: '',
    type: 'stdio',
    cmd: '',
    endpoint: '',
    enabled: true,
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
    type: extension.type,
    cmd: extension.type === 'stdio' ? combineCmdAndArgs(extension.cmd, extension.args) : undefined,
    endpoint: extension.type === 'sse' ? extension.uri : undefined,
    enabled: extension.enabled,
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
      cmd: cmd,
      args: args,
      ...(Object.keys(envs).length > 0 ? { envs } : {}),
    };
  } else if (formData.type === 'sse') {
    return {
      type: 'sse',
      name: formData.name,
      uri: formData.endpoint, // Assuming endpoint maps to uri for SSE type
      ...(Object.keys(envs).length > 0 ? { envs } : {}),
    };
  } else {
    // For other types
    return {
      type: formData.type,
      name: formData.name,
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
