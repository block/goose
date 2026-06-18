/**
 * @vitest-environment jsdom
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';

import AppSettingsSection from './AppSettingsSection';
import { IntlTestWrapper } from '../../../i18n/test-utils';

vi.mock('../../GooseSidebar/ThemeSelector', () => ({
  default: () => <div data-testid="theme-selector" />,
}));

vi.mock('./TelemetrySettings', () => ({
  default: () => <div data-testid="telemetry-settings" />,
}));

vi.mock('./UpdateSection', () => ({
  default: () => <div data-testid="update-section" />,
}));

function mockAppConfig(values: Record<string, unknown>) {
  (window as unknown as Record<string, unknown>).appConfig = {
    get: (key: string) => values[key],
    getAll: () => values,
  };
}

function mockElectron() {
  const electron = window.electron as unknown as Record<string, unknown>;
  Object.assign(electron, {
    platform: 'darwin',
    getMenuBarIconState: vi.fn(async () => true),
    getDockIconState: vi.fn(async () => true),
    getWakelockState: vi.fn(async () => false),
    getSetting: vi.fn(async (key: string) => (key === 'showPricing' ? true : true)),
    openNotificationsSettings: vi.fn(async () => true),
    setMenuBarIcon: vi.fn(async () => true),
    setDockIcon: vi.fn(async () => true),
    setWakelock: vi.fn(async () => true),
    setSetting: vi.fn(async () => undefined),
  });
}

describe('AppSettingsSection', () => {
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

  it('hides the cost tracking setting when Security Goose disables Token Plan pricing UI', async () => {
    mockAppConfig({
      GOOSE_VERSION: 'development',
      SECURITY_MODEL_PRICING_MODE: 'disabled-token-plan',
    });

    render(<AppSettingsSection />, { wrapper: IntlTestWrapper });

    expect(screen.queryByText('Cost Tracking')).not.toBeInTheDocument();
  });

  it('keeps the cost tracking setting visible when pricing mode is enabled', async () => {
    mockAppConfig({
      GOOSE_VERSION: 'development',
      SECURITY_MODEL_PRICING_MODE: 'enabled',
    });

    render(<AppSettingsSection />, { wrapper: IntlTestWrapper });

    expect(await screen.findByText('Cost Tracking')).toBeInTheDocument();
  });
});
