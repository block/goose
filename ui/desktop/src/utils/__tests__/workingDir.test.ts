import { beforeEach, describe, expect, it, vi } from 'vitest';
import { getEffectiveWorkingDir, resolveWorkingDir } from '../workingDir';

describe('resolveWorkingDir', () => {
  it('uses the configured external backend directory when present', () => {
    expect(resolveWorkingDir(' /home/goose ', 'C:\\Users\\goose', 'C:\\Users\\goose')).toBe(
      '/home/goose'
    );
    expect(resolveWorkingDir(' ', 'C:\\work', 'C:\\Users\\goose')).toBe('C:\\work');
    expect(resolveWorkingDir(undefined, undefined, 'C:\\Users\\goose')).toBe('C:\\Users\\goose');
  });
});

describe('getEffectiveWorkingDir', () => {
  const getSettingMock = vi.fn();
  const appConfigGetMock = vi.fn();

  beforeEach(() => {
    getSettingMock.mockReset();
    appConfigGetMock.mockReset();
    (globalThis as Record<string, unknown>).window = {
      appConfig: { get: appConfigGetMock },
      electron: { getSetting: getSettingMock },
    } as unknown as typeof globalThis;
  });

  it('prefers the configured remote directory when the external backend is enabled', async () => {
    appConfigGetMock.mockReturnValue('/Users/johannes/home/workspace');
    getSettingMock.mockResolvedValue({ enabled: true, workingDir: ' /home/goose/workspace ' });
    await expect(getEffectiveWorkingDir()).resolves.toBe('/home/goose/workspace');
  });

  it('falls back to the remembered directory when the external backend is disabled', async () => {
    appConfigGetMock.mockReturnValue('/Users/johannes/home/workspace');
    getSettingMock.mockResolvedValue({ enabled: false, workingDir: '/home/goose/workspace' });
    await expect(getEffectiveWorkingDir()).resolves.toBe('/Users/johannes/home/workspace');
  });

  it('falls back to the remembered directory when the remote directory is blank', async () => {
    appConfigGetMock.mockReturnValue('/Users/johannes/home/workspace');
    getSettingMock.mockResolvedValue({ enabled: true, workingDir: '   ' });
    await expect(getEffectiveWorkingDir()).resolves.toBe('/Users/johannes/home/workspace');
  });

  it('falls back to the remembered directory when the setting cannot be read', async () => {
    appConfigGetMock.mockReturnValue('/Users/johannes/home/workspace');
    getSettingMock.mockRejectedValue(new Error('settings unavailable'));
    await expect(getEffectiveWorkingDir()).resolves.toBe('/Users/johannes/home/workspace');
  });
});
