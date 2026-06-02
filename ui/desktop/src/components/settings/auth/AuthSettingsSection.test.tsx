import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, type RenderOptions } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import AuthSettingsSection from './AuthSettingsSection';
import {
  configureProviderOauth,
  deleteProviderSecret,
  listProviderSecrets,
  ProviderSecret,
} from '../../../api';
import { IntlTestWrapper } from '../../../i18n/test-utils';
import { toast } from 'react-toastify';

vi.mock('../../../api', async () => {
  const actual = await vi.importActual<typeof import('../../../api')>('../../../api');
  return {
    ...actual,
    configureProviderOauth: vi.fn(),
    listProviderSecrets: vi.fn(),
    deleteProviderSecret: vi.fn(),
  };
});

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

const mockedListProviderSecrets = vi.mocked(listProviderSecrets);
const mockedDeleteProviderSecret = vi.mocked(deleteProviderSecret);
const mockedConfigureProviderOauth = vi.mocked(configureProviderOauth);
const mockedToast = vi.mocked(toast);

const renderWithIntl = (ui: React.ReactElement, options?: RenderOptions) =>
  render(ui, { wrapper: IntlTestWrapper, ...options });

const providerSecret: ProviderSecret = {
  id: 'secret_store:openai:OPENAI_API_KEY',
  provider: 'openai',
  provider_display_name: 'OpenAI',
  name: 'OPENAI_API_KEY',
  storage: 'secret_store',
  expires_at: null,
  status: 'unknown',
  configured: true,
  has_secret: true,
  can_delete: true,
  can_configure: false,
  configure_provider: null,
};

const apiResult = <T,>(data: T) => ({
  data,
  request: {} as never,
  response: {} as never,
});

describe('AuthSettingsSection', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedListProviderSecrets.mockResolvedValue(apiResult({ secrets: [] }));
    mockedDeleteProviderSecret.mockResolvedValue(apiResult('ok'));
    mockedConfigureProviderOauth.mockResolvedValue(apiResult('ok'));
  });

  it('renders an empty state when no credentials are stored', async () => {
    renderWithIntl(<AuthSettingsSection />);

    expect(screen.getByText('Loading credentials...')).toBeInTheDocument();
    expect(await screen.findByText('No locally stored provider credentials were found.')).toBeInTheDocument();
  });

  it('renders provider credentials with storage and expiry status', async () => {
    mockedListProviderSecrets.mockResolvedValue(
      apiResult({
        secrets: [
          {
            ...providerSecret,
            expires_at: '2027-01-01T12:00:00Z',
            status: 'valid',
          },
        ],
      })
    );

    renderWithIntl(<AuthSettingsSection />);

    expect(await screen.findByText('OpenAI')).toBeInTheDocument();
    expect(screen.getByText('OPENAI_API_KEY')).toBeInTheDocument();
    expect(screen.getByText('Secret store')).toBeInTheDocument();
    expect(screen.getByText(/Expires/)).toBeInTheDocument();
  });

  it('does not render an expiry badge when expiry is unknown', async () => {
    mockedListProviderSecrets.mockResolvedValue(apiResult({ secrets: [providerSecret] }));

    renderWithIntl(<AuthSettingsSection />);

    expect(await screen.findByText('OpenAI')).toBeInTheDocument();
    expect(screen.getByText('Secret store')).toBeInTheDocument();
    expect(screen.queryByText('Expiry unknown')).not.toBeInTheDocument();
    expect(screen.queryByText(/Expires/)).not.toBeInTheDocument();
  });

  it('deletes a credential after confirmation and refreshes the list', async () => {
    const user = userEvent.setup();
    mockedListProviderSecrets
      .mockResolvedValueOnce(apiResult({ secrets: [providerSecret] }))
      .mockResolvedValueOnce(apiResult({ secrets: [] }));

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
      expect(mockedDeleteProviderSecret).toHaveBeenCalledWith({
        path: { id: 'secret_store:openai:OPENAI_API_KEY' },
        throwOnError: true,
      });
    });
    await waitFor(() => {
      expect(mockedToast.success).toHaveBeenCalledWith('Credential deleted');
    });
    expect(await screen.findByText('No locally stored provider credentials were found.')).toBeInTheDocument();
  });

  it('configures the permanent Hugging Face credential row', async () => {
    const user = userEvent.setup();
    const huggingFaceSecret: ProviderSecret = {
      id: 'provider_cache:huggingface',
      provider: 'huggingface',
      provider_display_name: 'Hugging Face',
      name: 'OAuth token',
      storage: 'provider_cache',
      expires_at: null,
      status: 'unknown',
      configured: false,
      has_secret: false,
      can_delete: false,
      can_configure: true,
      configure_provider: 'huggingface',
    };

    mockedListProviderSecrets
      .mockResolvedValueOnce(apiResult({ secrets: [huggingFaceSecret] }))
      .mockResolvedValueOnce(
        apiResult({
          secrets: [
            {
              ...huggingFaceSecret,
              configured: true,
              has_secret: true,
              can_delete: true,
            },
          ],
        })
      );

    renderWithIntl(<AuthSettingsSection />);

    expect(await screen.findByText('Hugging Face')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Delete credential' })).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Sign in' }));

    await waitFor(() => {
      expect(mockedConfigureProviderOauth).toHaveBeenCalledWith({
        path: { name: 'huggingface' },
        throwOnError: true,
      });
    });
    await waitFor(() => {
      expect(mockedToast.success).toHaveBeenCalledWith('Credential configured');
    });
  });
});
