import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createSession } from '../sessions';
import type { ExtensionConfig, Session } from '../api';
import type { FixedExtensionEntry } from '../components/ConfigContext';
import type { GooseExtension, GooseExtensionEntry } from '@aaif/goose-sdk';
import { getConfiguredGooseExtensions } from '../acp/extensions';
import { acpChatSessionController } from '../acp/chatSessionController';

vi.mock('../acp/extensions', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../acp/extensions')>();
  return {
    ...actual,
    getConfiguredGooseExtensions: vi.fn(),
  };
});

vi.mock('../acp/chatSessionController', () => ({
  acpChatSessionController: {
    createSession: vi.fn(),
  },
}));

const testSession: Session = {
  id: 'session-1',
  name: 'untitled',
  message_count: 0,
  created_at: '2026-06-19T00:00:00.000Z',
  updated_at: '2026-06-19T00:00:00.000Z',
  working_dir: '/tmp',
  extension_data: { active: [], installed: [] },
};

const extensionConfig = (name: string): ExtensionConfig => ({
  name,
  type: 'builtin',
  description: `${name} extension`,
});

const configuredExtension = (name: string, enabled: boolean): FixedExtensionEntry => ({
  ...extensionConfig(name),
  enabled,
});

const gooseExtension = (name: string): GooseExtension => ({
  type: 'builtin',
  name,
  description: `${name} extension`,
});

const gooseExtensionEntry = (name: string): GooseExtensionEntry => ({
  extension: gooseExtension(name),
  enabled: true,
});

const mockedGetConfiguredGooseExtensions = vi.mocked(getConfiguredGooseExtensions);
const mockedCreateAcpSession = vi.mocked(acpChatSessionController.createSession);

describe('createSession ACP session extensions', () => {
  beforeEach(() => {
    mockedGetConfiguredGooseExtensions.mockReset();
    mockedGetConfiguredGooseExtensions.mockResolvedValue([
      gooseExtensionEntry('developer'),
      gooseExtensionEntry('memory'),
    ]);
    mockedCreateAcpSession.mockReset();
    mockedCreateAcpSession.mockResolvedValue(testSession);
  });

  it('sends non-empty extension configs as ACP session extensions', async () => {
    await createSession('/tmp', {
      extensionConfigs: [extensionConfig('developer')],
    });

    expect(mockedGetConfiguredGooseExtensions).toHaveBeenCalledOnce();
    expect(mockedCreateAcpSession).toHaveBeenCalledWith('/tmp', [gooseExtension('developer')], {
      recipeDeeplink: undefined,
      recipeId: undefined,
    });
  });

  it('falls back to enabled configured extensions when extension configs are empty', async () => {
    await createSession('/tmp', {
      extensionConfigs: [],
      allExtensions: [configuredExtension('developer', true), configuredExtension('memory', false)],
    });

    expect(mockedGetConfiguredGooseExtensions).toHaveBeenCalledOnce();
    expect(mockedCreateAcpSession).toHaveBeenCalledWith('/tmp', [gooseExtension('developer')], {
      recipeDeeplink: undefined,
      recipeId: undefined,
    });
  });

  it('omits ACP session extensions when no configured extensions are enabled', async () => {
    await createSession('/tmp', {
      allExtensions: [configuredExtension('developer', false)],
    });

    expect(mockedGetConfiguredGooseExtensions).not.toHaveBeenCalled();
    expect(mockedCreateAcpSession).toHaveBeenCalledWith('/tmp', [], {
      recipeDeeplink: undefined,
      recipeId: undefined,
    });
  });
});
