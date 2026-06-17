/**
 * @vitest-environment jsdom
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';

import UpdateSection from './UpdateSection';
import { IntlTestWrapper } from '../../../i18n/test-utils';

function mockAppConfig(values: Record<string, unknown>) {
  (window as unknown as Record<string, unknown>).appConfig = {
    get: (key: string) => values[key],
    getAll: () => values,
  };
}

function mockElectron() {
  const electron = window.electron as unknown as Record<string, unknown>;
  Object.assign(electron, {
    getVersion: vi.fn(() => '1.37.0'),
    getUpdateState: vi.fn(async () => null),
    isUsingGitHubFallback: vi.fn(async () => false),
    onUpdaterEvent: vi.fn(),
    checkForUpdates: vi.fn(async () => ({ updateInfo: null, error: null })),
    installUpdate: vi.fn(),
  });
}

describe('UpdateSection', () => {
  beforeEach(() => {
    mockElectron();
  });

  afterEach(() => {
    cleanup();
    (window as unknown as Record<string, unknown>).appConfig = {
      get: () => undefined,
      getAll: () => ({}),
    };
  });

  it('disables release update checks for packaged local preview sessions', async () => {
    mockAppConfig({
      SECURITY_PREVIEW_SESSION_MODE: 'packaged-preview-explicit',
      SECURITY_UPDATER_MODE: 'local-preview-disabled',
      SECURITY_UPDATER_DISABLED_REASON: 'local-preview',
    });

    render(<UpdateSection />, { wrapper: IntlTestWrapper });

    expect(
      await screen.findByText('Local preview builds do not check signed release updates.')
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Check for Updates' })).toBeDisabled();
  });

  it('keeps manual update checks available for standard desktop sessions', async () => {
    mockAppConfig({
      SECURITY_PREVIEW_SESSION_MODE: 'standard',
      SECURITY_UPDATER_MODE: 'enabled',
    });

    render(<UpdateSection />, { wrapper: IntlTestWrapper });

    expect(
      screen.queryByText('Local preview builds do not check signed release updates.')
    ).not.toBeInTheDocument();
    expect(await screen.findByRole('button', { name: 'Check for Updates' })).toBeEnabled();
  });
});
