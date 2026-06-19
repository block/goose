import { beforeEach, describe, expect, it, vi } from 'vitest';
import { startAgent } from '../api';
import { createSession } from '../sessions';
import type { ExtensionConfig, Session } from '../api';
import type { FixedExtensionEntry } from '../components/ConfigContext';

vi.mock('../api', () => ({
  startAgent: vi.fn(),
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

const mockedStartAgent = vi.mocked(startAgent);

describe('createSession extension overrides', () => {
  beforeEach(() => {
    mockedStartAgent.mockReset();
    mockedStartAgent.mockResolvedValue({
      data: testSession,
      error: undefined,
      request: new globalThis.Request('http://localhost/sessions'),
      response: new globalThis.Response(),
    });
  });

  it('sends non-empty extension configs as overrides', async () => {
    await createSession('/tmp', {
      extensionConfigs: [extensionConfig('developer')],
    });

    expect(mockedStartAgent).toHaveBeenCalledWith({
      body: {
        working_dir: '/tmp',
        extension_overrides: [extensionConfig('developer')],
      },
      throwOnError: true,
    });
  });

  it('falls back to enabled configured extensions when extension configs are empty', async () => {
    await createSession('/tmp', {
      extensionConfigs: [],
      allExtensions: [configuredExtension('developer', true), configuredExtension('memory', false)],
    });

    expect(mockedStartAgent).toHaveBeenCalledWith({
      body: {
        working_dir: '/tmp',
        extension_overrides: [extensionConfig('developer')],
      },
      throwOnError: true,
    });
  });

  it('omits extension overrides when no configured extensions are enabled', async () => {
    await createSession('/tmp', {
      allExtensions: [configuredExtension('developer', false)],
    });

    expect(mockedStartAgent).toHaveBeenCalledWith({
      body: {
        working_dir: '/tmp',
      },
      throwOnError: true,
    });
  });
});
