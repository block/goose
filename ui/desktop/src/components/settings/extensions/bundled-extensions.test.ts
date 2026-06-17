import path from 'node:path';
import { afterEach, beforeEach, describe, it, expect, vi } from 'vitest';
import { pruneDeprecatedBundledExtensions, syncBundledExtensions } from './bundled-extensions';
import type { FixedExtensionEntry } from '../../ConfigContext';

vi.mock('./bundled-extensions.json', () => ({
  default: [
    {
      id: 'developer',
      name: 'developer',
      display_name: 'Developer',
      description: 'General development tools.',
      enabled: true,
      type: 'builtin',
      timeout: 300,
    },
    {
      id: 'googledrive',
      name: 'googledrive',
      display_name: 'Google Drive',
      description: 'Google Drive integration.',
      enabled: true,
      type: 'stdio',
      cmd: 'googledrive-mcp',
      args: [],
      env_keys: [],
      timeout: 300,
    },
    {
      id: 'browser-assist-mcp',
      name: 'browser-assist-mcp',
      display_name: 'Browser Assist',
      description: 'Security preview browser extension.',
      enabled: false,
      type: 'stdio',
      cmd: 'node',
      args: ['distro/security-cn/extensions/browser-assist-mcp/server.mjs'],
      env_keys: [],
      timeout: 300,
    },
  ],
}));

vi.mock('./deprecated-bundled-extensions.json', () => ({
  default: [{ id: 'googledrive' }, { id: 'old-bundled-extension' }],
}));

const MOCK_DISTRO_DIR = path.join('/mock', 'repo', 'distro', 'security-cn');
const MOCK_NODE_CMD = path.join('/mock', 'repo', 'ui', 'desktop', 'electron');

beforeEach(() => {
  (window as unknown as Record<string, unknown>).appConfig = {
    get: (key: string) => {
      if (key === 'GOOSE_DISTRO_DIR') {
        return MOCK_DISTRO_DIR;
      }
      if (key === 'GOOSE_DESKTOP_STDIO_NODE_CMD') {
        return MOCK_NODE_CMD;
      }
      return undefined;
    },
    getAll: () => ({
      GOOSE_DISTRO_DIR: MOCK_DISTRO_DIR,
      GOOSE_DESKTOP_STDIO_NODE_CMD: MOCK_NODE_CMD,
    }),
  };
});

afterEach(() => {
  delete (window as unknown as Record<string, unknown>).appConfig;
});

describe('syncBundledExtensions', () => {
  it('skips already bundled non-deprecated extensions', async () => {
    const addExtensionFn = vi.fn().mockResolvedValue(undefined);
    const existingExtensions = [
      {
        name: 'developer',
        type: 'builtin',
        description: 'General development tools.',
        display_name: 'Developer',
        enabled: true,
        bundled: true,
        timeout: 300,
      },
    ] as FixedExtensionEntry[];

    await syncBundledExtensions(existingExtensions, addExtensionFn);

    expect(addExtensionFn).not.toHaveBeenCalledWith(
      'developer',
      expect.anything(),
      expect.anything()
    );
  });

  it('resolves security stdio server args to absolute repo paths before syncing', async () => {
    const addExtensionFn = vi.fn().mockResolvedValue(undefined);

    await syncBundledExtensions([], addExtensionFn);

    const browserAssistCall = addExtensionFn.mock.calls.find(
      ([name]) => name === 'browser-assist-mcp'
    );

    expect(browserAssistCall).toBeDefined();
    expect(browserAssistCall?.[1]).toMatchObject({
      type: 'stdio',
      cmd: MOCK_NODE_CMD,
      envs: {
        ELECTRON_RUN_AS_NODE: '1',
      },
    });
    const browserAssistArgs = (browserAssistCall?.[1] as { args?: string[] }).args ?? [];
    expect(browserAssistArgs).toHaveLength(1);
    expect(browserAssistArgs[0]).toBe(
      path.join(MOCK_DISTRO_DIR, 'extensions', 'browser-assist-mcp', 'server.mjs')
    );
    expect(browserAssistCall?.[2]).toBe(false);
  });
});

describe('pruneDeprecatedBundledExtensions', () => {
  it('removes deprecated bundled extensions', async () => {
    const removeExtensionFn = vi.fn().mockResolvedValue(undefined);
    const existingExtensions = [
      {
        name: 'old-bundled-extension',
        type: 'builtin',
        description: 'Old bundled extension',
        enabled: true,
        bundled: true,
      },
    ] as FixedExtensionEntry[];

    const remainingExtensions = await pruneDeprecatedBundledExtensions(
      existingExtensions,
      removeExtensionFn
    );

    expect(removeExtensionFn).toHaveBeenCalledWith('old-bundled-extension');
    expect(remainingExtensions).toEqual([]);
  });

  it('does not remove non-bundled deprecated extensions', async () => {
    const removeExtensionFn = vi.fn().mockResolvedValue(undefined);
    const existingExtensions = [
      {
        name: 'old-bundled-extension',
        type: 'builtin',
        description: 'Old bundled extension',
        enabled: true,
        bundled: false,
      },
    ] as FixedExtensionEntry[];

    const remainingExtensions = await pruneDeprecatedBundledExtensions(
      existingExtensions,
      removeExtensionFn
    );

    expect(removeExtensionFn).not.toHaveBeenCalled();
    expect(remainingExtensions).toEqual(existingExtensions);
  });

  it('allows same-id bundled extensions to be re-added after prune', async () => {
    const removeExtensionFn = vi.fn().mockResolvedValue(undefined);
    const addExtensionFn = vi.fn().mockResolvedValue(undefined);
    const existingExtensions = [
      {
        name: 'Google Drive',
        type: 'stdio',
        description: 'Google Drive extension',
        cmd: 'some-cmd',
        args: [],
        env_keys: [],
        enabled: true,
        bundled: true,
      },
    ] as FixedExtensionEntry[];

    const remainingExtensions = await pruneDeprecatedBundledExtensions(
      existingExtensions,
      removeExtensionFn
    );

    await syncBundledExtensions(remainingExtensions, addExtensionFn);

    expect(removeExtensionFn).toHaveBeenCalledWith('googledrive');
    expect(addExtensionFn).toHaveBeenCalledWith(
      'googledrive',
      expect.objectContaining({
        type: 'stdio',
        name: 'googledrive',
        bundled: true,
      }),
      true
    );
  });
});
