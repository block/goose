import { describe, expect, it } from 'vitest';

import { resolveDesktopReleaseRepository, resolveDesktopUpdateMode } from './updateMode';

describe('resolveDesktopUpdateMode', () => {
  it('disables updater setup for repo preview sessions', () => {
    expect(
      resolveDesktopUpdateMode({
        previewSessionMode: 'repo-preview',
        updatesEnabled: true,
        enableDevUpdates: false,
      })
    ).toMatchObject({
      mode: 'local-preview-disabled',
      disabledReason: 'local-preview',
      canCheckForUpdates: false,
      shouldSetupUpdater: false,
    });
  });

  it('disables updater setup for packaged preview sessions even when updates are globally enabled', () => {
    expect(
      resolveDesktopUpdateMode({
        previewSessionMode: 'packaged-preview-explicit',
        updatesEnabled: true,
        enableDevUpdates: true,
      })
    ).toMatchObject({
      mode: 'local-preview-disabled',
      disabledReason: 'local-preview',
      canCheckForUpdates: false,
      shouldSetupUpdater: false,
    });
  });

  it('keeps signed and standard sessions update-capable when updates are enabled', () => {
    expect(
      resolveDesktopUpdateMode({
        previewSessionMode: 'standard',
        updatesEnabled: true,
        enableDevUpdates: false,
      })
    ).toMatchObject({
      mode: 'enabled',
      disabledReason: null,
      canCheckForUpdates: true,
      shouldSetupUpdater: true,
    });
  });
});

describe('resolveDesktopReleaseRepository', () => {
  it('uses the canonical Forge defaults when no env override is present', () => {
    expect(resolveDesktopReleaseRepository({})).toEqual({
      owner: 'aaif-goose',
      repo: 'goose',
    });
  });

  it('honors explicit repository overrides', () => {
    expect(
      resolveDesktopReleaseRepository({
        GITHUB_OWNER: 'Ramos-dev',
        GITHUB_REPO: 'security-goose',
      })
    ).toEqual({
      owner: 'Ramos-dev',
      repo: 'security-goose',
    });
  });
});
