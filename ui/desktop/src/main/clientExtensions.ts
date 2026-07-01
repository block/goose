import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { app } from 'electron';
import { compareVersions, satisfies } from 'compare-versions';
import packageJson from '../../package.json';
import {
  CLIENT_EXTENSION_MANIFEST,
  type ChatActionContribution,
  type ClientExtensionManifest,
  type ContentSuffixContribution,
  type CustomRenderContribution,
  type CustomRenderMatch,
  type DiscoveredClientExtension,
  type RootLinkContribution,
  type SidecarContribution,
} from '../client-extensions/types';

const MANIFEST_FILENAME = CLIENT_EXTENSION_MANIFEST;

function userClientExtensionsDir(): string {
  return path.join(os.homedir(), '.agents', 'client-extensions');
}

export function getClientExtensionsInstallDir(): string {
  return userClientExtensionsDir();
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function devClientExtensionsDir(): string | null {
  if (app.isPackaged) {
    return null;
  }

  const candidates = [
    path.join(process.cwd(), 'examples', 'client-extensions'),
    path.join(process.cwd(), '..', '..', 'examples', 'client-extensions'),
  ];

  for (const candidate of candidates) {
    if (fs.existsSync(candidate) && fs.statSync(candidate).isDirectory()) {
      return candidate;
    }
  }

  return null;
}

function parseChatActions(raw: unknown): ChatActionContribution[] | undefined {
  if (!Array.isArray(raw)) {
    return undefined;
  }

  const chatActions = raw
    .map((entry) => {
      if (!isRecord(entry)) {
        return null;
      }
      if (typeof entry.id !== 'string' || typeof entry.label !== 'string') {
        return null;
      }
      return {
        id: entry.id,
        label: entry.label,
        when: typeof entry.when === 'string' ? entry.when : undefined,
      };
    })
    .filter((entry): entry is NonNullable<typeof entry> => entry !== null);

  return chatActions.length > 0 ? chatActions : undefined;
}

function parseRootLinks(raw: unknown): RootLinkContribution[] | undefined {
  if (!Array.isArray(raw)) {
    return undefined;
  }

  const rootLinks = raw
    .map((entry) => {
      if (!isRecord(entry)) {
        return null;
      }
      if (typeof entry.id !== 'string' || typeof entry.label !== 'string') {
        return null;
      }
      return {
        id: entry.id,
        label: entry.label,
        when: typeof entry.when === 'string' ? entry.when : undefined,
      };
    })
    .filter((entry): entry is NonNullable<typeof entry> => entry !== null);

  return rootLinks.length > 0 ? rootLinks : undefined;
}

function parseContentSuffixes(raw: unknown): ContentSuffixContribution[] | undefined {
  if (!Array.isArray(raw)) {
    return undefined;
  }

  const contentSuffixes = raw
    .map((entry) => {
      if (!isRecord(entry)) {
        return null;
      }
      if (typeof entry.id !== 'string') {
        return null;
      }
      return {
        id: entry.id,
        when: typeof entry.when === 'string' ? entry.when : undefined,
      };
    })
    .filter((entry): entry is NonNullable<typeof entry> => entry !== null);

  return contentSuffixes.length > 0 ? contentSuffixes : undefined;
}

function parseCustomRenderMatch(raw: unknown): CustomRenderMatch | null {
  if (!isRecord(raw)) {
    return null;
  }

  const match: CustomRenderMatch = {};

  if (typeof raw.contentType === 'string') {
    if (raw.contentType === 'code' || raw.contentType === 'text') {
      match.contentType = raw.contentType;
    }
  }

  if (typeof raw.language === 'string' && raw.language.trim()) {
    match.language = raw.language.trim().toLowerCase();
  }

  return Object.keys(match).length > 0 ? match : null;
}

function parseCustomRenders(raw: unknown): CustomRenderContribution[] | undefined {
  if (!Array.isArray(raw)) {
    return undefined;
  }

  const customRenders = raw
    .map((entry) => {
      if (!isRecord(entry)) {
        return null;
      }
      if (typeof entry.id !== 'string') {
        return null;
      }

      const match = parseCustomRenderMatch(entry.match);
      if (!match) {
        return null;
      }

      const render: CustomRenderContribution = {
        id: entry.id,
        match,
      };

      if (typeof entry.when === 'string') {
        render.when = entry.when;
      }
      if (entry.display === 'inline') {
        render.display = 'inline';
      }
      if (typeof entry.priority === 'number' && Number.isFinite(entry.priority)) {
        render.priority = entry.priority;
      }

      return render;
    })
    .filter((entry): entry is NonNullable<typeof entry> => entry !== null);

  return customRenders.length > 0 ? customRenders : undefined;
}

function parseSidecars(raw: unknown): SidecarContribution[] | undefined {
  if (!Array.isArray(raw)) {
    return undefined;
  }

  const sidecars = raw
    .map((entry) => {
      if (!isRecord(entry)) {
        return null;
      }
      if (typeof entry.id !== 'string' || typeof entry.label !== 'string') {
        return null;
      }

      const sidecar: SidecarContribution = {
        id: entry.id,
        label: entry.label,
      };

      if (typeof entry.when === 'string') {
        sidecar.when = entry.when;
      }
      if (entry.defaultOpen === true) {
        sidecar.defaultOpen = true;
      }

      return sidecar;
    })
    .filter((entry): entry is NonNullable<typeof entry> => entry !== null);

  return sidecars.length > 0 ? sidecars : undefined;
}

function parseManifest(raw: unknown, rootPath: string): ClientExtensionManifest | null {
  if (!isRecord(raw)) {
    return null;
  }

  const id = raw.id;
  const version = raw.version;
  const main = raw.main;
  if (typeof id !== 'string' || !id.trim()) {
    return null;
  }
  if (typeof version !== 'string' || !version.trim()) {
    return null;
  }
  if (typeof main !== 'string' || !main.trim()) {
    return null;
  }

  const manifest: ClientExtensionManifest = {
    id: id.trim(),
    version: version.trim(),
    main: main.trim(),
  };

  if (isRecord(raw.engines) && typeof raw.engines.grc === 'string') {
    manifest.engines = { grc: raw.engines.grc };
  }

  if (isRecord(raw.contributes)) {
    const chatActions = parseChatActions(raw.contributes.chatActions);
    const rootLinks = parseRootLinks(raw.contributes.rootLinks);
    const contentSuffixes = parseContentSuffixes(raw.contributes.contentSuffixes);
    const customRenders = parseCustomRenders(raw.contributes.customRenders);
    const sidecars = parseSidecars(raw.contributes.sidecars);
    if (chatActions || rootLinks || contentSuffixes || customRenders || sidecars) {
      manifest.contributes = {
        ...(chatActions ? { chatActions } : {}),
        ...(rootLinks ? { rootLinks } : {}),
        ...(contentSuffixes ? { contentSuffixes } : {}),
        ...(customRenders ? { customRenders } : {}),
        ...(sidecars ? { sidecars } : {}),
      };
    }
  }

  const manifestPath = path.join(rootPath, MANIFEST_FILENAME);
  if (path.basename(rootPath) !== id && manifest.id !== path.basename(rootPath)) {
    console.warn(
      `[client-extensions] id "${manifest.id}" does not match directory "${path.basename(rootPath)}" at ${manifestPath}`
    );
  }

  return manifest;
}

function satisfiesGrcEngine(manifest: ClientExtensionManifest): boolean {
  const constraint = manifest.engines?.grc;
  if (!constraint) {
    return true;
  }

  const grcVersion = packageJson.version;
  try {
    if (/^[<>=]/.test(constraint) || constraint.includes(' ')) {
      return satisfies(grcVersion, constraint);
    }
    return compareVersions(grcVersion, constraint) >= 0;
  } catch {
    console.warn(`[client-extensions] Invalid engines.grc constraint "${constraint}"`);
    return false;
  }
}

function discoverInDirectory(extensionsRoot: string): DiscoveredClientExtension[] {
  if (!fs.existsSync(extensionsRoot)) {
    return [];
  }

  const discovered: DiscoveredClientExtension[] = [];

  for (const entry of fs.readdirSync(extensionsRoot, { withFileTypes: true })) {
    if (!entry.isDirectory()) {
      continue;
    }

    const rootPath = path.join(extensionsRoot, entry.name);
    const manifestPath = path.join(rootPath, MANIFEST_FILENAME);

    try {
      if (!fs.existsSync(manifestPath)) {
        continue;
      }

      const raw = JSON.parse(fs.readFileSync(manifestPath, 'utf8')) as unknown;
      const manifest = parseManifest(raw, rootPath);
      if (!manifest) {
        console.warn(`[client-extensions] Invalid manifest at ${manifestPath}`);
        continue;
      }

      if (!satisfiesGrcEngine(manifest)) {
        console.warn(
          `[client-extensions] Skipping "${manifest.id}": requires GRC ${manifest.engines?.grc}, running ${packageJson.version}`
        );
        continue;
      }

      const mainPath = path.join(rootPath, manifest.main);
      if (!fs.existsSync(mainPath)) {
        console.warn(`[client-extensions] Skipping "${manifest.id}": main entry missing at ${mainPath}`);
        continue;
      }

      discovered.push({ id: manifest.id, rootPath, manifest });
    } catch (error) {
      console.warn(`[client-extensions] Failed to load ${manifestPath}:`, error);
    }
  }

  return discovered;
}

export function discoverClientExtensions(): DiscoveredClientExtension[] {
  const roots = new Set<string>([userClientExtensionsDir()]);
  const devRoot = devClientExtensionsDir();
  if (devRoot) {
    roots.add(devRoot);
  }

  const byId = new Map<string, DiscoveredClientExtension>();
  for (const root of roots) {
    for (const extension of discoverInDirectory(root)) {
      byId.set(extension.id, extension);
    }
  }

  return [...byId.values()].sort((a, b) => a.id.localeCompare(b.id));
}

export function readClientExtensionMain(extensionId: string): string | null {
  const extension = discoverClientExtensions().find((entry) => entry.id === extensionId);
  if (!extension) {
    return null;
  }

  const mainPath = path.join(extension.rootPath, extension.manifest.main);
  try {
    return fs.readFileSync(mainPath, 'utf8');
  } catch (error) {
    console.warn(`[client-extensions] Failed to read main for "${extensionId}":`, error);
    return null;
  }
}
