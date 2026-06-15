import { createRequire } from 'node:module';
import { Buffer } from 'node:buffer';
import { describe, expect, it } from 'vitest';

const require = createRequire(import.meta.url);
const { inspectMacosSigningReadiness } = require('../scripts/lib/macosSigningReadiness.cjs');

describe('inspectMacosSigningReadiness', () => {
  it('treats default local bundle flows as unsigned local-preview with a clear fallback', () => {
    expect(inspectMacosSigningReadiness({})).toMatchObject({
      signingRequested: false,
      requestedMode: 'local-preview',
      readyForSignedRelease: false,
      missingSecrets: expect.arrayContaining([
        'APPLE_CERTIFICATE_BASE64',
        'APPLE_TEAM_ID',
        'APPLE_ID',
      ]),
    });
  });

  it('reports the exact blockers when signed release mode is requested without Apple secrets', () => {
    const result = inspectMacosSigningReadiness({
      GOOSE_DESKTOP_SIGN: 'true',
      APPLE_TEAM_ID: 'TEAM123',
    });

    expect(result).toMatchObject({
      signingRequested: true,
      requestedMode: 'signed',
      readyForSignedRelease: false,
    });
    expect(result.missingSecrets).toEqual(
      expect.arrayContaining([
        'APPLE_CERTIFICATE_BASE64',
        'APPLE_CERTIFICATE_PASSWORD',
        'APPLE_ID',
        'APPLE_ID_PASSWORD',
      ])
    );
    expect(result.blockerSummary).toContain('missing secrets:');
  });

  it('treats a full secret set as ready for signed release rehearsal', () => {
    expect(
      inspectMacosSigningReadiness({
        GOOSE_DESKTOP_SIGN: 'true',
        APPLE_CERTIFICATE_BASE64: Buffer.from('demo-certificate').toString('base64'),
        APPLE_CERTIFICATE_PASSWORD: 'secret',
        APPLE_TEAM_ID: 'TEAM123',
        APPLE_ID: 'signing@example.com',
        APPLE_ID_PASSWORD: 'app-specific-password',
      })
    ).toMatchObject({
      signingRequested: true,
      requestedMode: 'signed',
      readyForSignedRelease: true,
      missingSecrets: [],
      invalidSecrets: [],
    });
  });
});
