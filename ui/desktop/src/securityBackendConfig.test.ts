import path from 'node:path';
import { describe, expect, it, vi } from 'vitest';
import fs from 'node:fs';

import {
  resolveSecurityPreviewSessionMode,
  isPackagedLocalPreviewBundle,
  resolveAdditionalGooseConfigFiles,
  resolveDesktopUserDataDir,
  resolveBackendSecretEnv,
  resolvePreviewGoosePathRoot,
} from './securityBackendConfig';

describe('resolvePreviewGoosePathRoot', () => {
  it('prefers an explicit path root override', () => {
    expect(resolvePreviewGoosePathRoot('/tmp/custom-root', '/workspace')).toBe(
      path.resolve('/tmp/custom-root')
    );
  });

  it('falls back to a repo-local preview goose path root', () => {
    expect(resolvePreviewGoosePathRoot(undefined, '/workspace')).toBe(
      path.resolve('/workspace/.preview/goose-path')
    );
  });

  it('returns undefined outside preview-style repo contexts', () => {
    expect(resolvePreviewGoosePathRoot()).toBeUndefined();
  });

  it('falls back to a packaged local-preview goose path root when no explicit env is provided', () => {
    expect(
      resolvePreviewGoosePathRoot(undefined, undefined, {
        isPackaged: true,
        existingEnv: { GOOSE_LOCAL_PREVIEW_BUNDLE: '1' },
        appName: 'Security Goose',
        homeDir: '/Users/tester',
      })
    ).toBe(
      path.resolve(
        '/Users/tester/.security-goose/security-goose/local-preview/user-data/goose-path'
      )
    );
  });
});

describe('isPackagedLocalPreviewBundle', () => {
  it('recognizes the packaged local-preview marker env var', () => {
    expect(
      isPackagedLocalPreviewBundle({
        isPackaged: true,
        existingEnv: { GOOSE_LOCAL_PREVIEW_BUNDLE: '1' },
      })
    ).toBe(true);
  });

  it('does not treat ordinary packaged apps as local-preview bundles', () => {
    expect(
      isPackagedLocalPreviewBundle({
        isPackaged: true,
        existingEnv: {},
      })
    ).toBe(false);
  });
});

describe('resolveDesktopUserDataDir', () => {
  it('prefers an explicit user data dir override', () => {
    expect(
      resolveDesktopUserDataDir({
        explicitValue: '~/custom-user-data',
        homeDir: '/Users/tester',
      })
    ).toBe(path.resolve('/Users/tester/custom-user-data'));
  });

  it('falls back to repo-local preview user data for repo preview runs', () => {
    expect(resolveDesktopUserDataDir({ previewRepoRoot: '/workspace' })).toBe(
      path.resolve('/workspace/.preview/user-data')
    );
  });

  it('falls back to a deterministic packaged local-preview user data dir', () => {
    expect(
      resolveDesktopUserDataDir({
        isPackaged: true,
        existingEnv: { GOOSE_LOCAL_PREVIEW_BUNDLE: '1' },
        appName: 'Security Goose',
        homeDir: '/Users/tester',
      })
    ).toBe(path.resolve('/Users/tester/.security-goose/security-goose/local-preview/user-data'));
  });
});

describe('resolveSecurityPreviewSessionMode', () => {
  it('classifies repo preview runs separately from packaged preview sessions', () => {
    expect(
      resolveSecurityPreviewSessionMode({
        previewRepoRoot: '/workspace',
        isPackaged: false,
      })
    ).toBe('repo-preview');
  });

  it('treats packaged local-preview launches with explicit isolation env as supported preview entries', () => {
    expect(
      resolveSecurityPreviewSessionMode({
        isPackaged: true,
        existingEnv: { GOOSE_LOCAL_PREVIEW_BUNDLE: '1' },
        explicitUserDataDir: '/tmp/preview-user-data',
        explicitGoosePathRoot: '/tmp/preview-goose-path',
        appName: 'Security Goose',
        homeDir: '/Users/tester',
      })
    ).toBe('packaged-preview-explicit');
  });

  it('treats packaged local-preview launches without explicit isolation env as fallback sessions', () => {
    expect(
      resolveSecurityPreviewSessionMode({
        isPackaged: true,
        existingEnv: { GOOSE_LOCAL_PREVIEW_BUNDLE: '1' },
        appName: 'Security Goose',
        homeDir: '/Users/tester',
        userDataDir: '/Users/tester/.security-goose/security-goose/local-preview/user-data',
      })
    ).toBe('packaged-preview-fallback');
  });
});

describe('resolveAdditionalGooseConfigFiles', () => {
  it('returns undefined when no backend config file exists', () => {
    const existsSync = vi.spyOn(fs, 'existsSync').mockReturnValue(false);

    expect(
      resolveAdditionalGooseConfigFiles({
        previewRepoRoot: '/workspace',
        workingDir: '/workspace/project',
      })
    ).toBeUndefined();

    existsSync.mockRestore();
  });

  it('deduplicates repo and working-directory init-config files and preserves override order', () => {
    const existsSync = vi.spyOn(fs, 'existsSync').mockImplementation((candidate) =>
      [
        path.resolve('/workspace/init-config.yaml'),
        path.resolve('/workspace/project/init-config.yaml'),
      ].includes(path.resolve(String(candidate)))
    );

    expect(
      resolveAdditionalGooseConfigFiles({
        previewRepoRoot: '/workspace',
        workingDir: '/workspace/project',
        existingValue: '/tmp/override-a.yaml:/tmp/override-b.yaml',
      })
    ).toBe(
      [
        path.resolve('/workspace/init-config.yaml'),
        path.resolve('/workspace/project/init-config.yaml'),
        path.resolve('/tmp/override-a.yaml'),
        path.resolve('/tmp/override-b.yaml'),
      ].join(path.delimiter)
    );

    existsSync.mockRestore();
  });

  it('avoids duplicate entries when the working directory is already the repo root', () => {
    const existsSync = vi.spyOn(fs, 'existsSync').mockImplementation((candidate) =>
      path.resolve(String(candidate)) === path.resolve('/workspace/init-config.yaml')
    );

    expect(
      resolveAdditionalGooseConfigFiles({
        previewRepoRoot: '/workspace',
        workingDir: '/workspace',
        existingValue: '/workspace/init-config.yaml',
      })
    ).toBe(path.resolve('/workspace/init-config.yaml'));

    existsSync.mockRestore();
  });
});

describe('resolveBackendSecretEnv', () => {
  it('reads missing backend secrets from local init-config files', () => {
    const existsSync = vi.spyOn(fs, 'existsSync').mockImplementation((candidate) =>
      [
        path.resolve('/workspace/init-config.yaml'),
        path.resolve('/workspace/project/init-config.yaml'),
      ].includes(path.resolve(String(candidate)))
    );
    const readFileSync = vi.spyOn(fs, 'readFileSync').mockImplementation((candidate) => {
      const resolved = path.resolve(String(candidate));
      if (resolved === path.resolve('/workspace/init-config.yaml')) {
        return 'OPENAI_API_KEY: repo-key\n';
      }
      if (resolved === path.resolve('/workspace/project/init-config.yaml')) {
        return 'OPENAI_API_KEY: project-key\n';
      }
      throw new Error(`unexpected file read: ${resolved}`);
    });

    expect(
      resolveBackendSecretEnv({
        previewRepoRoot: '/workspace',
        workingDir: '/workspace/project',
        secretKeys: ['OPENAI_API_KEY'],
        existingEnv: {},
      })
    ).toEqual({ OPENAI_API_KEY: 'project-key' });

    readFileSync.mockRestore();
    existsSync.mockRestore();
  });

  it('does not override an explicit backend secret env var', () => {
    const existsSync = vi.spyOn(fs, 'existsSync').mockReturnValue(true);
    const readFileSync = vi.spyOn(fs, 'readFileSync').mockReturnValue('OPENAI_API_KEY: file-key\n');

    expect(
      resolveBackendSecretEnv({
        previewRepoRoot: '/workspace',
        secretKeys: ['OPENAI_API_KEY'],
        existingEnv: { OPENAI_API_KEY: 'env-key' },
      })
    ).toEqual({});

    readFileSync.mockRestore();
    existsSync.mockRestore();
  });
});
