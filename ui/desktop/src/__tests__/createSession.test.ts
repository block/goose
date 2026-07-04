import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createSession } from '../sessions';
import type { ExtensionConfig } from '../types/extensions';
import type { Session } from '../types/session';
import type { FixedExtensionEntry } from '../components/ConfigContext';
import type { GoslingExtension, GoslingExtensionEntry } from '@repo-makeover/gosling-sdk';
import { getConfiguredGoslingExtensions } from '../acp/extensions';
import { acpChatSessionController } from '../acp/chatSessionController';

vi.mock('../acp/extensions', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../acp/extensions')>();
  return {
    ...actual,
    getConfiguredGoslingExtensions: vi.fn(),
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

const goslingExtension = (name: string): GoslingExtension => ({
  type: 'builtin',
  name,
  description: `${name} extension`,
});

const goslingExtensionEntry = (name: string): GoslingExtensionEntry => ({
  extension: goslingExtension(name),
  enabled: true,
});

const mockedGetConfiguredGoslingExtensions = vi.mocked(getConfiguredGoslingExtensions);
const mockedCreateAcpSession = vi.mocked(acpChatSessionController.createSession);

describe('createSession ACP session extensions', () => {
  beforeEach(() => {
    mockedGetConfiguredGoslingExtensions.mockReset();
    mockedGetConfiguredGoslingExtensions.mockResolvedValue([
      goslingExtensionEntry('developer'),
      goslingExtensionEntry('memory'),
    ]);
    mockedCreateAcpSession.mockReset();
    mockedCreateAcpSession.mockResolvedValue(testSession);
  });

  it('sends non-empty extension configs as ACP session extensions', async () => {
    await createSession('/tmp', {
      extensionConfigs: [extensionConfig('developer')],
    });

    expect(mockedGetConfiguredGoslingExtensions).toHaveBeenCalledOnce();
    expect(mockedCreateAcpSession).toHaveBeenCalledWith('/tmp', [goslingExtension('developer')]);
  });

  it('falls back to enabled configured extensions when extension configs are empty', async () => {
    await createSession('/tmp', {
      extensionConfigs: [],
      allExtensions: [configuredExtension('developer', true), configuredExtension('memory', false)],
    });

    expect(mockedGetConfiguredGoslingExtensions).toHaveBeenCalledOnce();
    expect(mockedCreateAcpSession).toHaveBeenCalledWith('/tmp', [goslingExtension('developer')]);
  });

  it('omits ACP session extensions when no configured extensions are enabled', async () => {
    await createSession('/tmp', {
      allExtensions: [configuredExtension('developer', false)],
    });

    expect(mockedGetConfiguredGoslingExtensions).not.toHaveBeenCalled();
    expect(mockedCreateAcpSession).toHaveBeenCalledWith('/tmp', []);
  });
});
