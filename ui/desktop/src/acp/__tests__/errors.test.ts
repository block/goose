import { describe, expect, it } from 'vitest';
import { isWorkingDirMissingError, parseAcpCreditsExhaustedError } from '../errors';

describe('parseAcpCreditsExhaustedError', () => {
  it('parses structured ACP credits exhausted errors', () => {
    expect(
      parseAcpCreditsExhaustedError({
        code: -32603,
        message: 'Please add credits to your account, then resend your message to continue.',
        data: {
          reason: 'credits_exhausted',
          url: 'https://router.tetrate.ai/billing',
        },
      })
    ).toEqual({
      message: 'Please add credits to your account, then resend your message to continue.',
      url: 'https://router.tetrate.ai/billing',
    });
  });

  it('parses wrapped JSON-RPC errors', () => {
    expect(
      parseAcpCreditsExhaustedError({
        error: {
          code: -32603,
          message: 'Add credits to continue.',
          data: {
            reason: 'credits_exhausted',
          },
        },
      })
    ).toEqual({
      message: 'Add credits to continue.',
    });
  });

  it('ignores non-credits-exhausted errors', () => {
    expect(
      parseAcpCreditsExhaustedError({
        code: -32603,
        message: 'Something failed.',
        data: {
          reason: 'provider_error',
        },
      })
    ).toBeNull();
  });
});

describe('isWorkingDirMissingError', () => {
  it('detects the structured working-dir-missing reason', () => {
    expect(
      isWorkingDirMissingError({
        code: -32602,
        message: 'Working directory no longer exists: /gone.',
        data: {
          reason: 'working_dir_missing',
          workingDir: '/gone',
        },
      })
    ).toBe(true);
  });

  it('detects the reason on wrapped JSON-RPC errors', () => {
    expect(
      isWorkingDirMissingError({
        error: { code: -32602, message: 'gone', data: { reason: 'working_dir_missing' } },
      })
    ).toBe(true);
  });

  it('ignores unrelated errors', () => {
    expect(isWorkingDirMissingError({ code: -32602, message: 'bad', data: {} })).toBe(false);
    expect(isWorkingDirMissingError(new Error('nope'))).toBe(false);
  });
});
