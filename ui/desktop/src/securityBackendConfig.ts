import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

interface ResolveAdditionalGooseConfigFilesOptions {
  existingValue?: string;
  previewRepoRoot?: string;
  workingDir?: string;
}

interface ResolveBackendSecretEnvOptions extends ResolveAdditionalGooseConfigFilesOptions {
  existingEnv?: Record<string, string | undefined>;
  secretKeys: string[];
}

interface ResolveDesktopUserDataDirOptions {
  explicitValue?: string;
  previewRepoRoot?: string;
  isPackaged?: boolean;
  existingEnv?: Record<string, string | undefined>;
  appName?: string;
  homeDir?: string;
}

interface ResolveGoosePathRootOptions {
  isPackaged?: boolean;
  existingEnv?: Record<string, string | undefined>;
  appName?: string;
  homeDir?: string;
  userDataDir?: string;
}

interface ResolveSecurityPreviewSessionModeOptions extends ResolveGoosePathRootOptions {
  explicitUserDataDir?: string;
  explicitGoosePathRoot?: string;
  previewRepoRoot?: string;
}

export type SecurityPreviewSessionMode =
  | 'standard'
  | 'repo-preview'
  | 'packaged-preview-explicit'
  | 'packaged-preview-fallback';

function normalizePathInput(value: string, homeDir: string): string {
  if (value === '~') {
    return homeDir;
  }

  if (value.startsWith(`~${path.sep}`)) {
    return path.join(homeDir, value.slice(2));
  }

  return value;
}

function slugifyAppName(appName?: string): string {
  return (appName || 'security-goose')
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

function getPackagedLocalPreviewFallbackUserDataDir(
  appName: string | undefined,
  homeDir: string
): string {
  return path.resolve(
    homeDir,
    '.security-goose',
    slugifyAppName(appName),
    'local-preview',
    'user-data'
  );
}

export function isPackagedLocalPreviewBundle(options: ResolveGoosePathRootOptions = {}): boolean {
  return Boolean(options.isPackaged && options.existingEnv?.GOOSE_LOCAL_PREVIEW_BUNDLE === '1');
}

export function resolveDesktopUserDataDir(
  options: ResolveDesktopUserDataDirOptions = {}
): string | undefined {
  const homeDir = options.homeDir ?? os.homedir();

  if (options.explicitValue?.trim()) {
    return path.resolve(normalizePathInput(options.explicitValue.trim(), homeDir));
  }

  if (options.previewRepoRoot?.trim()) {
    return path.resolve(options.previewRepoRoot, '.preview', 'user-data');
  }

  if (!isPackagedLocalPreviewBundle(options)) {
    return undefined;
  }

  return getPackagedLocalPreviewFallbackUserDataDir(options.appName, homeDir);
}

export function resolvePreviewGoosePathRoot(
  explicitValue?: string,
  previewRepoRoot?: string,
  options: ResolveGoosePathRootOptions = {}
): string | undefined {
  const homeDir = options.homeDir ?? os.homedir();

  if (explicitValue?.trim()) {
    return path.resolve(normalizePathInput(explicitValue.trim(), homeDir));
  }

  if (!previewRepoRoot?.trim()) {
    if (!isPackagedLocalPreviewBundle(options)) {
      return undefined;
    }

    const userDataDir =
      options.userDataDir ??
      resolveDesktopUserDataDir({
        isPackaged: options.isPackaged,
        existingEnv: options.existingEnv,
        appName: options.appName,
        homeDir,
      });

    return userDataDir ? path.resolve(userDataDir, 'goose-path') : undefined;
  }

  return path.resolve(previewRepoRoot, '.preview', 'goose-path');
}

export function resolveSecurityPreviewSessionMode(
  options: ResolveSecurityPreviewSessionModeOptions = {}
): SecurityPreviewSessionMode {
  if (options.previewRepoRoot?.trim()) {
    return 'repo-preview';
  }

  if (!isPackagedLocalPreviewBundle(options)) {
    return 'standard';
  }

  if (options.explicitUserDataDir?.trim() && options.explicitGoosePathRoot?.trim()) {
    return 'packaged-preview-explicit';
  }

  return 'packaged-preview-fallback';
}

function splitConfigFileList(value?: string): string[] {
  if (!value?.trim()) {
    return [];
  }

  return value
    .split(path.delimiter)
    .map((entry) => entry.trim())
    .filter((entry) => entry.length > 0);
}

function resolveInitConfigPath(root?: string): string | undefined {
  if (!root?.trim()) {
    return undefined;
  }

  const candidate = path.resolve(root, 'init-config.yaml');
  return fs.existsSync(candidate) ? candidate : undefined;
}

function listResolvedConfigFiles(options: ResolveAdditionalGooseConfigFilesOptions = {}): string[] {
  const entries = [
    resolveInitConfigPath(options.previewRepoRoot),
    resolveInitConfigPath(options.workingDir),
    ...splitConfigFileList(options.existingValue),
  ].filter((entry): entry is string => typeof entry === 'string' && entry.length > 0);

  return Array.from(new Set(entries.map((entry) => path.resolve(entry))));
}

function readTopLevelScalar(filePath: string, key: string): string | undefined {
  let contents: string;
  try {
    contents = fs.readFileSync(filePath, 'utf8');
  } catch {
    return undefined;
  }

  for (const rawLine of contents.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith('#')) {
      continue;
    }

    const separatorIndex = line.indexOf(':');
    if (separatorIndex <= 0) {
      continue;
    }

    const candidateKey = line.slice(0, separatorIndex).trim();
    if (candidateKey !== key) {
      continue;
    }

    return line
      .slice(separatorIndex + 1)
      .trim()
      .replace(/^['"]|['"]$/g, '');
  }

  return undefined;
}

export function resolveAdditionalGooseConfigFiles(
  options: ResolveAdditionalGooseConfigFilesOptions = {}
): string | undefined {
  const deduped = listResolvedConfigFiles(options);
  return deduped.length > 0 ? deduped.join(path.delimiter) : undefined;
}

export function resolveBackendSecretEnv(
  options: ResolveBackendSecretEnvOptions
): Record<string, string> {
  const resolved: Record<string, string> = {};
  const existingEnv = options.existingEnv ?? process.env;
  const configFiles = listResolvedConfigFiles(options);

  for (const key of options.secretKeys) {
    if (existingEnv[key]?.trim()) {
      continue;
    }

    for (const filePath of configFiles) {
      const value = readTopLevelScalar(filePath, key);
      if (value?.trim()) {
        resolved[key] = value;
      }
    }
  }

  return resolved;
}
