import path from 'node:path';
import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  getGoosedBinaryCandidatePaths,
  isAllowedGoosedBinaryPath,
  waitForInitialFingerprint,
} from './goosed';

describe('goosed binary path resolution', () => {
  const cwd = '/workspace/ui/desktop';

  it('prefers repo-built development candidates before the staged desktop copy', () => {
    expect(getGoosedBinaryCandidatePaths({ cwd, isPackaged: false })).toEqual([
      path.resolve('/workspace/target/release/goosed'),
      path.resolve('/workspace/target/debug/goosed'),
      path.resolve('/workspace/ui/desktop/src/bin/goosed'),
    ]);
  });

  it('allows only repo-owned development overrides in preview', () => {
    expect(isAllowedGoosedBinaryPath('/workspace/target/debug/goosed', { cwd })).toBe(true);
    expect(isAllowedGoosedBinaryPath('/Applications/Goose.app/Contents/Resources/bin/goosed', { cwd })).toBe(false);
  });

  it('allows packaged bundle resources when packaged', () => {
    expect(
      isAllowedGoosedBinaryPath('/bundle/Resources/bin/goosed', {
        cwd,
        isPackaged: true,
        resourcesPath: '/bundle/Resources',
      })
    ).toBe(true);
  });
});

describe('waitForInitialFingerprint', () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it('returns the fingerprint when stdout emits it before the timeout', async () => {
    vi.useFakeTimers();
    const fingerprintPromise = new Promise<string>((resolve) => {
      setTimeout(() => resolve('AA:BB'), 10);
    });

    const waitPromise = waitForInitialFingerprint(fingerprintPromise, { timeoutMs: 1000 });

    await vi.advanceTimersByTimeAsync(10);

    await expect(waitPromise).resolves.toBe('AA:BB');
  });

  it('falls back to null after the timeout so desktop bootstrap can continue', async () => {
    vi.useFakeTimers();
    const logger = { info: vi.fn(), error: vi.fn() };
    const fingerprintPromise = new Promise<string | null>(() => {});

    const waitPromise = waitForInitialFingerprint(fingerprintPromise, {
      timeoutMs: 25,
      logger,
    });

    await vi.advanceTimersByTimeAsync(25);

    await expect(waitPromise).resolves.toBeNull();
    expect(logger.info).toHaveBeenCalledWith(
      'Timed out waiting 25ms for goosed TLS fingerprint on stdout, continuing with TOFU bootstrap'
    );
  });
});
