import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { acpRefreshProviderDetails, acpSaveProviderConfig } from '../../../../acp/providers';
import { IntlTestWrapper } from '../../../../i18n/test-utils';
import type { ProviderDetails } from '../../../../types/providers';
import ProviderConfigurationModal from './ProviderConfigurationModal';

vi.mock('../../../ModelAndProviderContext', () => ({
  useModelAndProvider: () => ({
    getCurrentModelAndProvider: vi.fn().mockResolvedValue({
      provider: 'another-provider',
      model: 'model',
    }),
  }),
}));

vi.mock('../../../../acp/providers', () => ({
  acpAuthenticateProvider: vi.fn(),
  acpDeleteCustomProvider: vi.fn(),
  acpDeleteProviderConfig: vi.fn(),
  acpRefreshProviderDetails: vi.fn(),
  acpSaveProviderConfig: vi.fn(),
  isAcpProvider: (provider: ProviderDetails) => provider.uses_acp,
}));

const oauthProvider: ProviderDetails = {
  name: 'github_copilot',
  is_configured: true,
  is_available: true,
  visible_in_setup: true,
  deprecated: false,
  provider_type: 'Builtin',
  setup_category: 'model',
  uses_acp: false,
  metadata: {
    name: 'github_copilot',
    display_name: 'GitHub Copilot',
    description: 'GitHub Copilot models',
    default_model: 'current',
    known_models: [],
    model_doc_link: '',
    config_keys: [
      {
        name: 'GITHUB_COPILOT_OAUTH',
        required: true,
        secret: true,
        oauth_flow: true,
      },
    ],
  },
};

describe('ProviderConfigurationModal', () => {
  it('offers to remove an existing OAuth configuration without an ACP readiness check', () => {
    render(
      <ProviderConfigurationModal provider={oauthProvider} onClose={vi.fn()} />,
      { wrapper: IntlTestWrapper }
    );

    expect(screen.getByRole('button', { name: 'Remove Configuration' })).toBeInTheDocument();
  });

  it('invalidates ACP readiness while checking again', async () => {
    const user = userEvent.setup();
    const acpProvider: ProviderDetails = {
      ...oauthProvider,
      name: 'claude-acp',
      setup_category: 'agent',
      uses_acp: true,
      metadata: {
        ...oauthProvider.metadata,
        name: 'claude-acp',
        config_keys: [],
        setup_steps: ['Install and authenticate Claude Code'],
      },
    };
    let finishSecondCheck:
      | ((value: Awaited<ReturnType<typeof acpRefreshProviderDetails>>) => void)
      | undefined;
    vi.mocked(acpRefreshProviderDetails)
      .mockResolvedValueOnce({
        provider: acpProvider,
        connectionChecked: true,
        readinessError: null,
      })
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            finishSecondCheck = resolve;
          })
      );

    render(
      <ProviderConfigurationModal provider={acpProvider} onClose={vi.fn()} onConfigured={vi.fn()} />,
      { wrapper: IntlTestWrapper }
    );

    expect(await screen.findByRole('button', { name: 'Choose model' })).toBeInTheDocument();
    expect(acpRefreshProviderDetails).toHaveBeenCalledTimes(1);

    await user.click(screen.getByRole('button', { name: 'Check again' }));
    expect(screen.queryByRole('button', { name: 'Choose model' })).not.toBeInTheDocument();

    finishSecondCheck?.({
      provider: acpProvider,
      connectionChecked: true,
      readinessError: 'Authentication expired',
    });
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Check again' })).not.toBeDisabled()
    );
  });

  it('enables and removes an available ACP provider without changing the adapter', async () => {
    const user = userEvent.setup();
    const onConfigured = vi.fn();
    const acpProvider: ProviderDetails = {
      ...oauthProvider,
      name: 'codex-acp',
      is_configured: false,
      setup_category: 'agent',
      uses_acp: true,
      metadata: {
        ...oauthProvider.metadata,
        name: 'codex-acp',
        config_keys: [],
        setup_steps: ['Install Codex ACP'],
      },
    };
    vi.mocked(acpRefreshProviderDetails).mockResolvedValue({
      provider: acpProvider,
      connectionChecked: true,
      readinessError: null,
    });

    const view = render(
      <ProviderConfigurationModal
        provider={acpProvider}
        onClose={vi.fn()}
        onConfigured={onConfigured}
      />,
      { wrapper: IntlTestWrapper }
    );

    await user.click(await screen.findByRole('button', { name: 'Choose model' }));
    expect(acpSaveProviderConfig).toHaveBeenCalledWith('codex-acp', []);
    expect(onConfigured).toHaveBeenCalledWith(
      expect.objectContaining({ name: 'codex-acp', is_configured: true })
    );

    view.unmount();
    vi.mocked(acpRefreshProviderDetails).mockResolvedValue({
      provider: { ...acpProvider, is_configured: true },
      connectionChecked: true,
      readinessError: null,
    });
    render(
      <ProviderConfigurationModal
        provider={{ ...acpProvider, is_configured: true }}
        onClose={vi.fn()}
        onConfigured={onConfigured}
      />,
      { wrapper: IntlTestWrapper }
    );
    expect(await screen.findByRole('button', { name: 'Remove Configuration' })).toBeInTheDocument();
  });
});
