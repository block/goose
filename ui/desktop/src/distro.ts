import * as fs from 'fs';
import * as path from 'path';
import { app } from 'electron';

interface DistroConfig {
  env?: Record<string, string>;
  features?: Record<string, unknown>;
}

let distroDir: string | null = null;
let distroConfig: DistroConfig = {};

function findDistroDir(): string | null {
  const externalPath = path.join(app.getPath('userData'), 'distro');
  if (fs.existsSync(path.join(externalPath, 'distro.json'))) {
    return externalPath;
  }

  const bundlePath = path.join(process.resourcesPath, 'distro');
  if (fs.existsSync(path.join(bundlePath, 'distro.json'))) {
    return bundlePath;
  }

  return null;
}

// Executes on import — must be imported before anything that reads process.env
distroDir = findDistroDir();
if (distroDir) {
  const configPath = path.join(distroDir, 'distro.json');
  distroConfig = JSON.parse(fs.readFileSync(configPath, 'utf-8'));

  if (distroConfig.env) {
    for (const [key, value] of Object.entries(distroConfig.env)) {
      process.env[key] = value;
    }
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
