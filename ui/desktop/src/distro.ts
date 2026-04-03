import * as fs from 'fs';
import * as path from 'path';

interface DistroConfig {
  env?: Record<string, string>;
  features?: Record<string, unknown>;
}

let distroDir: string | null = null;
let distroConfig: DistroConfig = {};

function findDistroDir(): string | null {
  try {
    const bundlePath = path.join(process.resourcesPath, 'distro');
    if (fs.existsSync(path.join(bundlePath, 'distro.json'))) {
      return bundlePath;
    }
  } catch {
    // process.resourcesPath may be undefined outside Electron
  }

  return null;
}

// Executes on import — must be imported before anything that reads process.env
distroDir = findDistroDir();
if (distroDir) {
  try {
    const configPath = path.join(distroDir, 'distro.json');
    distroConfig = JSON.parse(fs.readFileSync(configPath, 'utf-8'));

    if (distroConfig.env) {
      for (const [key, value] of Object.entries(distroConfig.env)) {
        process.env[key] = value;
      }
    }
  } catch (err) {
    console.error('Failed to load distro.json, falling back to stock defaults:', err);
    distroDir = null;
    distroConfig = {};
  }
}

export function getDistroFeature<T>(key: string, defaultValue: T): T {
  if (distroConfig.features && key in distroConfig.features) {
    return distroConfig.features[key] as T;
  }
  return defaultValue;
}

export function getDistroDir(): string | null {
  return distroDir;
}

export function getDistroFilePath(relativePath: string): string | null {
  if (!distroDir) return null;
  const fullPath = path.join(distroDir, relativePath);
  return fs.existsSync(fullPath) ? fullPath : null;
}
