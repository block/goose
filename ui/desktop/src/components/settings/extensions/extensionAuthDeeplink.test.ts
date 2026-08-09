import { describe, expect, it } from 'vitest';
import {
  buildExtensionAuthenticateLink,
  isExtensionAuthenticateDeepLink,
  parseExtensionAuthenticateDeepLink,
} from './extensionAuthDeeplink';

describe('extensionAuthDeeplink', () => {
  it('builds avocado-work authenticate links', () => {
    expect(buildExtensionAuthenticateLink('google-workspace')).toBe(
      'avocado-work://extension-authenticate?configKey=google-workspace'
    );
  });

  it('includes force when requested', () => {
    expect(buildExtensionAuthenticateLink('google-workspace', { force: true })).toBe(
      'avocado-work://extension-authenticate?configKey=google-workspace&force=true'
    );
  });

  it('detects extension-authenticate host links', () => {
    expect(
      isExtensionAuthenticateDeepLink(
        'avocado-work://extension-authenticate?configKey=google-workspace'
      )
    ).toBe(true);
  });

  it('detects legacy action=authenticate links', () => {
    expect(
      isExtensionAuthenticateDeepLink(
        'goose://extension?action=authenticate&configKey=google-workspace'
      )
    ).toBe(true);
  });

  it('parses configKey and force', () => {
    expect(
      parseExtensionAuthenticateDeepLink(
        'avocado-work://extension-authenticate?configKey=google-workspace&force=true'
      )
    ).toEqual({ configKey: 'google-workspace', force: true });
  });

  it('falls back to id and name', () => {
    expect(
      parseExtensionAuthenticateDeepLink('goose://extension-authenticate?id=google-workspace')
    ).toEqual({ configKey: 'google-workspace', force: false });

    expect(
      parseExtensionAuthenticateDeepLink('goose://extension-authenticate?name=Google Workspace')
    ).toEqual({ configKey: 'googleworkspace', force: false });
  });
});
