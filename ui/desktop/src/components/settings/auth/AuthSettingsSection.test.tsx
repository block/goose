import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, type RenderOptions } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import AuthSettingsSection from './AuthSettingsSection';
import {
  acpAuthenticateProvider,
  acpDeleteProviderSecret,
  acpListProviderSecrets,
  type ProviderSecretDto,
} from '../../../acp/providers';
import { IntlTestWrapper } from '../../../i18n/test-utils';
import { toast } from 'react-toastify';

vi.mock('../../../acp/providers', () => ({
  acpAuthenticateProvider: vi.fn(),
  acpListProviderSecrets: vi.fn(),
  acpDeleteProviderSecret: vi.fn(),
}));

vi.mock('../../ModelAndProviderContext', () => ({
  useModelAndProvider: () => ({
    currentProvider: 'openai',
  }),
}));

vi.mock('react-toastify', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

const mockedListProviderSecrets = vi.mocked(acpListProviderSecrets);
const mockedDeleteProviderSecret = vi.mocked(acpDeleteProviderSecret);
const mockedAcpAuthenticateProvider = vi.mocked(acpAuthenticateProvider);
const mockedToast = vi.mocked(toast);

const renderWithIntl = (ui: React.ReactElement, options?: RenderOptions) =>
  render(ui, { wrapper: IntlTestWrapper, ...options });

const providerSecret: ProviderSecretDto = {
  id: 'secret_store:openai:OPENAI_API_KEY',
  provider: 'openai',
  providerDisplayName: 'OpenAI',
  name: 'OPENAI_API_KEY',
  storage: 'secret_store',
  expiresAt: null,
  status: 'unknown',
  configured: true,
  hasSecret: true,
  canDelete: true,
  canConfigure: false,
  configureProvider: null,
};

describe('AuthSettingsSection', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedListProviderSecrets.mockResolvedValue([]);
    mockedDeleteProviderSecret.mockResolvedValue(undefined);
    mockedAcpAuthenticateProvider.mockResolvedValue(undefined);
  });

  it('renders an empty state when no credentials are stored', async () => {
    renderWithIntl(<AuthSettingsSection />);

    expect(screen.getByText('Loading credentials...')).toBeInTheDocument();
    expect(await screen.findByText('No locally stored provider credentials were found.')).toBeInTheDocument();
  });

  it('renders provider credentials with storage and expiry status', async () => {
    mockedListProviderSecrets.mockResolvedValue([
      {
        ...providerSecret,
        expiresAt: '2027-01-01T12:00:00Z',
        status: 'valid',
      },
    ]);

    renderWithIntl(<AuthSettingsSection />);

    expect(await screen.findByText('OpenAI')).toBeInTheDocument();
    expect(screen.getByText('OPENAI_API_KEY')).toBeInTheDocument();
    expect(screen.getByText('Secret store')).toBeInTheDocument();
    expect(screen.getByText(/Expires/)).toBeInTheDocument();
  });

  it('does not render an expiry badge when expiry is unknown', async () => {
    mockedListProviderSecrets.mockResolvedValue([providerSecret]);

    renderWithIntl(<AuthSettingsSection />);

    expect(await screen.findByText('OpenAI')).toBeInTheDocument();
    expect(screen.getByText('Secret store')).toBeInTheDocument();
    expect(screen.queryByText('Expiry unknown')).not.toBeInTheDocument();
    expect(screen.queryByText(/Expires/)).not.toBeInTheDocument();
  });

  it('deletes a credential after confirmation and refreshes the list', async () => {
    const user = userEvent.setup();
    mockedListProviderSecrets
      .mockResolvedValueOnce([providerSecret])
      .mockResolvedValueOnce([]);

    renderWithIntl(<AuthSettingsSection />);

    expect(await screen.findByText('OpenAI')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Delete credential' }));

    expect(screen.getByText('Delete the OPENAI_API_KEY credential for OpenAI?')).toBeInTheDocument();
    expect(
      screen.getByText(
        'This is the active provider. New requests may fail until you configure another credential.'
      )
    ).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Delete' }));

    await waitFor(() => {
      expect(mockedDeleteProviderSecret).toHaveBeenCalledWith('secret_store:openai:OPENAI_API_KEY');
    });
    await waitFor(() => {
      expect(mockedToast.success).toHaveBeenCalledWith('Credential deleted');
    });
    expect(await screen.findByText('No locally stored provider credentials were found.')).toBeInTheDocument();
  });

  it('configures the permanent Hugging Face credential row', async () => {
    const user = userEvent.setup();
    const huggingFaceSecret: ProviderSecretDto = {
      id: 'provider_cache:huggingface',
      provider: 'huggingface',
      providerDisplayName: 'Hugging Face',
      name: 'OAuth token',
      storage: 'provider_cache',
      expiresAt: null,
      status: 'unknown',
      configured: false,
      hasSecret: false,
      canDelete: false,
      canConfigure: true,
      configureProvider: 'huggingface',
    };

    mockedListProviderSecrets
      .mockResolvedValueOnce([huggingFaceSecret])
      .mockResolvedValueOnce([
        {
          ...huggingFaceSecret,
          configured: true,
          hasSecret: true,
          canDelete: true,
        },
      ]);

    renderWithIntl(<AuthSettingsSection />);

    expect(await screen.findByText('Hugging Face')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Delete credential' })).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Sign in' }));

    await waitFor(() => {
      expect(mockedAcpAuthenticateProvider).toHaveBeenCalledWith('huggingface');
    });
    await waitFor(() => {
      expect(mockedToast.success).toHaveBeenCalledWith('Credential configured');
    });
  });
});
