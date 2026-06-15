import fs from 'node:fs';
import path from 'node:path';

const DEFAULT_DISTRO_DIR = path.join('distro', 'security-cn');
const PROCESS_WITH_RESOURCES = process as typeof process & { resourcesPath?: string };

export interface SecurityDistroDefaults {
  distroDir?: string;
  distributionId?: string;
  bundleId?: string;
  productName: string;
  productNameZh?: string;
  locale?: string;
  defaultProvider?: string;
  defaultModel?: string;
  predefinedModels: string;
}

interface ProductMetadata {
  distributionId?: string;
  bundleId?: string;
  productName?: string;
  productNameZh?: string;
  defaultLocale?: string;
}

export function parseEnvFile(contents: string): Record<string, string> {
  const parsed: Record<string, string> = {};

  for (const rawLine of contents.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith('#')) {
      continue;
    }

    const separatorIndex = line.indexOf('=');
    if (separatorIndex <= 0) {
      continue;
    }

    const key = line.slice(0, separatorIndex).trim();
    let value = line.slice(separatorIndex + 1).trim();

    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }

    parsed[key] = value;
  }

  return parsed;
}

function fileExists(filePath: string): boolean {
  try {
    return fs.statSync(filePath).isFile();
  } catch {
    return false;
  }
}

function readJsonFile<T>(filePath: string): T | undefined {
  if (!fileExists(filePath)) {
    return undefined;
  }

  return JSON.parse(fs.readFileSync(filePath, 'utf8')) as T;
}

function readTextFile(filePath: string): string | undefined {
  if (!fileExists(filePath)) {
    return undefined;
  }

  return fs.readFileSync(filePath, 'utf8');
}

function getSearchRoots(searchRoot?: string): string[] {
  const roots = [
    searchRoot,
    process.cwd(),
    path.resolve(process.cwd(), '../..'),
    __dirname,
    path.resolve(__dirname, '../../..'),
    path.resolve(__dirname, '../../../../'),
    PROCESS_WITH_RESOURCES.resourcesPath,
  ].filter((value): value is string => typeof value === 'string' && value.length > 0);

  return Array.from(new Set(roots));
}

function getCandidateDistroDirs(searchRoot?: string): string[] {
  return Array.from(
    new Set(
      getSearchRoots(searchRoot).flatMap((root) => [
        root,
        path.join(root, 'security-cn'),
        path.join(root, DEFAULT_DISTRO_DIR),
      ])
    )
  );
}

export function findDistroDir(searchRoot?: string): string | undefined {
  return getCandidateDistroDirs(searchRoot).find((candidateDir) =>
    fileExists(path.join(candidateDir, 'branding', 'product-metadata.json'))
  );
}

export function loadSecurityDistroDefaults(searchRoot?: string): SecurityDistroDefaults {
  const distroDir = findDistroDir(searchRoot);
  const metadata = distroDir
    ? readJsonFile<ProductMetadata>(path.join(distroDir, 'branding', 'product-metadata.json'))
    : undefined;
  const desktopEnv = distroDir
    ? parseEnvFile(readTextFile(path.join(distroDir, 'config', 'desktop-env.example')) ?? '')
    : {};
  const modelCatalog =
    distroDir &&
    readJsonFile<Array<Record<string, unknown>>>(
      path.join(distroDir, 'config', 'model-catalog.json')
    );

  return {
    distroDir,
    distributionId: metadata?.distributionId,
    bundleId: metadata?.bundleId,
    productName: metadata?.productName?.trim() || 'Goose',
    productNameZh: metadata?.productNameZh?.trim(),
    locale: desktopEnv.GOOSE_LOCALE || metadata?.defaultLocale,
    defaultProvider: desktopEnv.GOOSE_DEFAULT_PROVIDER,
    defaultModel: desktopEnv.GOOSE_DEFAULT_MODEL,
    predefinedModels:
      desktopEnv.GOOSE_PREDEFINED_MODELS ||
      JSON.stringify(Array.isArray(modelCatalog) ? modelCatalog : []),
  };
}
