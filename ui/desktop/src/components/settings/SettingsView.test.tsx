/**
 * @vitest-environment jsdom
 */
import { render, screen, waitFor } from '@testing-library/react';
import { vi, describe, expect, it } from 'vitest';
import SettingsView from './SettingsView';
import { IntlTestWrapper } from '../../i18n/test-utils';

vi.mock('../../api/sdk.gen', () => ({
  getTunnelStatus: vi.fn().mockResolvedValue({ data: { state: 'running' } }),
}));

vi.mock('../../utils/analytics', () => ({
  trackSettingsTabViewed: vi.fn(),
}));

vi.mock('../Layout/MainPanelLayout', () => ({
  MainPanelLayout: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));

vi.mock('./models/ModelsSection', () => ({
  default: () => <div>Models section</div>,
}));

vi.mock('./chat/ChatSettingsSection', () => ({
  default: () => <div>Chat section</div>,
}));

vi.mock('./sessions/SessionSharingSection', () => ({
  default: () => <div>Session sharing section</div>,
}));

vi.mock('./app/ExternalBackendSection', () => ({
  default: () => <div>External backend section</div>,
}));

vi.mock('./tunnel/TunnelSection', () => ({
  default: () => <div>Tunnel section</div>,
}));

vi.mock('./gateways/GatewaySettingsSection', () => ({
  default: () => <div>Gateway settings section</div>,
}));

vi.mock('./PromptsSettingsSection', () => ({
  default: () => <div>Prompts section</div>,
}));

vi.mock('./keyboard/KeyboardShortcutsSection', () => ({
  default: () => <div>Keyboard section</div>,
}));

vi.mock('./app/AppSettingsSection', () => ({
  default: () => <div>App settings section</div>,
}));

vi.mock('./config/ConfigSettings', () => ({
  default: () => <div>Config settings section</div>,
}));

describe('SettingsView', () => {
  it('hides local inference and mesh settings tabs from ApeCloud builds', async () => {
    render(
      <SettingsView
        onClose={vi.fn()}
        setView={vi.fn()}
        viewOptions={{ section: 'local-inference' }}
      />,
      { wrapper: IntlTestWrapper }
    );

    await waitFor(() => {
      expect(screen.getByTestId('settings-models-tab')).toBeInTheDocument();
    });

    expect(screen.queryByTestId('settings-local-inference-tab')).not.toBeInTheDocument();
    expect(screen.queryByTestId('settings-mesh-tab')).not.toBeInTheDocument();
    expect(screen.getByText('Models section')).toBeInTheDocument();
  });
});
