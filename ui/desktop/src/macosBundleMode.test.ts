import { createRequire } from 'node:module';
import { describe, expect, it } from 'vitest';

const require = createRequire(import.meta.url);
const { resolveMacosBundleMode } = require('../scripts/lib/macosBundleMode.cjs');

describe('resolveMacosBundleMode', () => {
  it('treats local bundle builds as unsigned preview even when ambient Apple env is present', () => {
    expect(
      resolveMacosBundleMode({
        APPLE_TEAM_ID: 'TEAM123',
        GOOSE_DESKTOP_SIGN: 'false',
      })
    ).toMatchObject({
      signingEnabled: false,
      signingMode: 'local-preview',
      enableCookieEncryption: false,
      disableKeyringByDefault: true,
      shouldAdhocResign: true,
    });
  });

  it('keeps signed release behavior when desktop signing is explicitly enabled', () => {
    expect(
      resolveMacosBundleMode({
        APPLE_TEAM_ID: 'TEAM123',
        GOOSE_DESKTOP_SIGN: 'true',
      })
    ).toMatchObject({
      signingEnabled: true,
      signingMode: 'signed',
      enableCookieEncryption: true,
      disableKeyringByDefault: false,
      shouldAdhocResign: false,
    });
  });
});
