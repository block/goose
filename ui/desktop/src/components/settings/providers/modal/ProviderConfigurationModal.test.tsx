import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
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

const oauthProvider: ProviderDetails = {
  name: 'github_copilot',
  is_configured: true,
  visible_in_setup: true,
  deprecated: false,
  provider_type: 'Builtin',
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
});
