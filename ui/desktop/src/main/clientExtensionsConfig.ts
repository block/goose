import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

export interface ClientExtensionsConfig {
  disabled: string[];
  enabledDev: string[];
}

export type ClientExtensionSource = 'installed' | 'dev';

const CONFIG_FILENAME = 'config.json';

export function getClientExtensionsInstallDir(): string {
  return path.join(os.homedir(), '.agents', 'client-extensions');
}

function configPath(): string {
  return path.join(getClientExtensionsInstallDir(), CONFIG_FILENAME);
}

export function defaultClientExtensionsConfig(): ClientExtensionsConfig {
  return { disabled: [], enabledDev: [] };
}

export function loadClientExtensionsConfig(): ClientExtensionsConfig {
  const filePath = configPath();
  try {
    if (!fs.existsSync(filePath)) {
      return defaultClientExtensionsConfig();
    }
    const raw = JSON.parse(fs.readFileSync(filePath, 'utf8')) as unknown;
    if (typeof raw !== 'object' || raw === null) {
      return defaultClientExtensionsConfig();
    }
    const record = raw as Record<string, unknown>;
    return {
      disabled: Array.isArray(record.disabled)
        ? record.disabled.filter((entry): entry is string => typeof entry === 'string')
        : [],
      enabledDev: Array.isArray(record.enabledDev)
        ? record.enabledDev.filter((entry): entry is string => typeof entry === 'string')
        : [],
    };
  } catch (error) {
    console.warn('[client-extensions] Failed to read config, using defaults:', error);
    return defaultClientExtensionsConfig();
  }
}

export function saveClientExtensionsConfig(config: ClientExtensionsConfig): void {
  const installDir = getClientExtensionsInstallDir();
  fs.mkdirSync(installDir, { recursive: true });
  fs.writeFileSync(configPath(), `${JSON.stringify(config, null, 2)}\n`, 'utf8');
}

export function isClientExtensionEnabled(
  id: string,
  source: ClientExtensionSource,
  config: ClientExtensionsConfig
): boolean {
  if (config.disabled.includes(id)) {
    return false;
  }
  if (source === 'dev') {
    return config.enabledDev.includes(id);
  }
  return true;
}

export function setClientExtensionEnabled(
  id: string,
  source: ClientExtensionSource,
  enabled: boolean
): ClientExtensionsConfig {
  const config = loadClientExtensionsConfig();
  const disabled = new Set(config.disabled);
  const enabledDev = new Set(config.enabledDev);

  if (enabled) {
    disabled.delete(id);
    if (source === 'dev') {
      enabledDev.add(id);
    }
  } else {
    disabled.add(id);
    if (source === 'dev') {
      enabledDev.delete(id);
    }
  }

  const next = {
    disabled: [...disabled].sort(),
    enabledDev: [...enabledDev].sort(),
  };
  saveClientExtensionsConfig(next);
  return next;
}
