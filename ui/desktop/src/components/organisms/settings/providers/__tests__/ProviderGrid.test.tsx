import { TooltipProvider } from '@radix-ui/react-tooltip';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, type Mock, vi } from 'vitest';
import type { ProviderDetails } from '@/api';
import ProviderGrid from '../ProviderGrid';

// ── Mocks ────────────────────────────────────────────────────────────────────

// Mock the API module
vi.mock('@/api', async () => {
  const actual = await vi.importActual('@/api');
  return {
    ...actual,
    getCustomProvider: vi.fn(),
    createCustomProvider: vi.fn(),
    updateCustomProvider: vi.fn(),
    deleteCustomProvider: vi.fn(),
  };
});

// Mock ModelAndProviderContext
const mockGetCurrentModelAndProvider = vi.fn();
const mockChangeModel = vi.fn();
vi.mock('@/contexts/ModelAndProviderContext', () => ({
  useModelAndProvider: () => ({
    getCurrentModelAndProvider: mockGetCurrentModelAndProvider,
    changeModel: mockChangeModel,
    getCurrentModelDisplayName: vi.fn().mockResolvedValue('test-model'),
    getCurrentProviderDisplayName: vi.fn().mockResolvedValue('Test Provider'),
    refreshCurrentModelAndProvider: vi.fn(),
    setProviderAndModel: vi.fn(),
    currentModel: 'test-model',
    currentProvider: 'test-provider',
  }),
}));

// Mock ConfigContext
vi.mock('@/contexts/ConfigContext', () => ({
  useConfig: () => ({
    read: vi.fn().mockResolvedValue(''),
    upsert: vi.fn().mockResolvedValue(undefined),
    getProviders: vi.fn().mockResolvedValue([]),
  }),
}));

// Mock SwitchModelModal (exact import path from ProviderGrid)
vi.mock('@/components/organisms/settings/models/subcomponents/SwitchModelModal', () => ({
  SwitchModelModal: ({
    onClose,
    initialProvider,
  }: {
    onClose: () => void;
    initialProvider?: string;
  }) => (
    <div data-testid="switch-model-modal">
      <span data-testid="switch-model-initial-provider">{initialProvider}</span>
      <button type="button" data-testid="switch-model-close" onClick={onClose}>
        Close
      </button>
    </div>
  ),
}));

// Mock ProviderConfigurationModal (exact path: ./modal/ProviderConfiguationModal)
vi.mock('@/components/organisms/settings/providers/modal/ProviderConfiguationModal', () => ({
  default: ({
    provider,
    onClose,
    onConfigured,
  }: {
    provider: ProviderDetails;
    onClose: () => void;
    onConfigured: (provider: ProviderDetails) => void;
  }) => (
    <div data-testid="provider-config-modal">
      <span data-testid="config-modal-provider">{provider.name}</span>
      <button type="button" data-testid="config-modal-close" onClick={onClose}>
        Close
      </button>
      <button
        type="button"
        data-testid="config-modal-configured"
        onClick={() => onConfigured(provider)}
      >
        Configured
      </button>
    </div>
  ),
}));

// Mock CustomProviderForm (exact path: ./modal/subcomponents/forms/CustomProviderForm)
vi.mock(
  '@/components/organisms/settings/providers/modal/subcomponents/forms/CustomProviderForm',
  () => ({
    default: ({
      onSubmit,
      onCancel,
      onDelete,
    }: {
      onSubmit: (data: unknown) => void;
      onCancel: () => void;
      onDelete?: () => void;
    }) => (
      <div data-testid="custom-provider-form">
        <button
          type="button"
          data-testid="custom-form-submit"
          onClick={() =>
            onSubmit({
              engine: 'openai',
              display_name: 'My Custom',
              api_url: 'https://api.example.com',
              api_key: 'key',
              models: ['model-1'],
              supports_streaming: true,
              requires_auth: true,
            })
          }
        >
          Submit
        </button>
        <button type="button" data-testid="custom-form-cancel" onClick={onCancel}>
          Cancel
        </button>
        {onDelete && (
          <button type="button" data-testid="custom-form-delete" onClick={onDelete}>
            Delete
          </button>
        )}
      </div>
    ),
  })
);

// Mock toast
vi.mock('@/components/molecules/ui/use-toast', () => ({
  useToast: () => ({ toast: vi.fn() }),
}));

// ── Helpers ──────────────────────────────────────────────────────────────────

function makeProvider(overrides: Partial<ProviderDetails> = {}): ProviderDetails {
  return {
    name: 'openai',
    is_configured: true,
    provider_type: 'BuiltIn',
    metadata: {
      display_name: 'OpenAI',
      description: 'OpenAI API',
      config_keys: [],
      default_model: 'gpt-4o',
      known_models: [],
      model_doc_link: '',
      name: 'openai',
    },
    ...overrides,
  } as ProviderDetails;
}

function makeCustomProvider(overrides: Partial<ProviderDetails> = {}): ProviderDetails {
  return makeProvider({
    name: 'my-custom',
    provider_type: 'Custom',
    metadata: {
      display_name: 'My Custom',
      description: 'Custom provider',
      config_keys: [],
      default_model: 'model-1',
      known_models: [],
      model_doc_link: '',
      name: 'my-custom',
    },
    ...overrides,
  });
}

function renderWithProviders(ui: React.ReactElement) {
  return render(<TooltipProvider>{ui}</TooltipProvider>);
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe('ProviderGrid', () => {
  const mockRefresh = vi.fn();
  const mockSetView = vi.fn();
  const mockOnModelSelected = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    mockGetCurrentModelAndProvider.mockResolvedValue({
      model: 'gpt-4o',
      provider: 'openai',
    });
  });

  describe('rendering', () => {
    it('renders provider cards and a custom provider card', () => {
      const providers = [
        makeProvider(),
        makeProvider({
          name: 'anthropic',
          metadata: {
            display_name: 'Anthropic',
            description: 'Anthropic API',
            config_keys: [],
            default_model: 'claude-sonnet-4-20250514',
            known_models: [],
            model_doc_link: '',
            name: 'anthropic',
          },
        }),
      ];

      renderWithProviders(
        <ProviderGrid
          providers={providers}
          isOnboarding={false}
          refreshProviders={mockRefresh}
          setView={mockSetView}
          onModelSelected={mockOnModelSelected}
        />
      );

      expect(screen.getByTestId('provider-card-openai')).toBeInTheDocument();
      expect(screen.getByTestId('provider-card-anthropic')).toBeInTheDocument();
      expect(screen.getByTestId('add-custom-provider-card')).toBeInTheDocument();
    });

    it('sorts providers alphabetically by display name', () => {
      const providers = [
        makeProvider({
          name: 'zeta',
          metadata: {
            display_name: 'Zeta Provider',
            description: '',
            config_keys: [],
            default_model: 'zeta-1',
            known_models: [],
            model_doc_link: '',
            name: 'zeta',
          },
        }),
        makeProvider({
          name: 'alpha',
          metadata: {
            display_name: 'Alpha Provider',
            description: '',
            config_keys: [],
            default_model: 'alpha-1',
            known_models: [],
            model_doc_link: '',
            name: 'alpha',
          },
        }),
      ];

      renderWithProviders(
        <ProviderGrid providers={providers} isOnboarding={false} refreshProviders={mockRefresh} />
      );

      const cards = screen.getAllByTestId(/^provider-card-/);
      expect(cards[0]).toHaveAttribute('data-testid', 'provider-card-alpha');
      expect(cards[1]).toHaveAttribute('data-testid', 'provider-card-zeta');
    });

    it('renders only the custom provider card when no providers exist', () => {
      renderWithProviders(
        <ProviderGrid providers={[]} isOnboarding={false} refreshProviders={mockRefresh} />
      );

      expect(screen.getByTestId('add-custom-provider-card')).toBeInTheDocument();
    });
  });

  describe('Add Custom Provider card click', () => {
    it('opens the custom provider dialog when clicked', async () => {
      renderWithProviders(
        <ProviderGrid providers={[]} isOnboarding={false} refreshProviders={mockRefresh} />
      );

      const customCard = screen.getByTestId('add-custom-provider-card');
      const button = customCard.querySelector('button[aria-label="Add a custom provider"]');
      expect(button).toBeInTheDocument();

      expect(button).toBeTruthy();
      fireEvent.click(button as HTMLElement);

      await waitFor(() => {
        expect(screen.getByTestId('custom-provider-form')).toBeInTheDocument();
      });
    });

    it('shows "Add Provider" title when creating a new custom provider', async () => {
      renderWithProviders(
        <ProviderGrid providers={[]} isOnboarding={false} refreshProviders={mockRefresh} />
      );

      const customCard = screen.getByTestId('add-custom-provider-card');
      const button = customCard.querySelector('button[aria-label="Add a custom provider"]');
      expect(button).toBeTruthy();
      fireEvent.click(button as HTMLElement);

      await waitFor(() => {
        // The dialog title is "Add Provider"
        expect(screen.getByRole('heading', { name: /Add\s+Provider/ })).toBeInTheDocument();
      });
    });

    it('calls createCustomProvider API and refreshes on submit', async () => {
      const { createCustomProvider } = await import('@/api');
      (createCustomProvider as Mock).mockResolvedValue({ data: {} });

      renderWithProviders(
        <ProviderGrid providers={[]} isOnboarding={false} refreshProviders={mockRefresh} />
      );

      const customCard = screen.getByTestId('add-custom-provider-card');
      const button = customCard.querySelector('button[aria-label="Add a custom provider"]');
      expect(button).toBeTruthy();
      fireEvent.click(button as HTMLElement);

      await waitFor(() => {
        expect(screen.getByTestId('custom-provider-form')).toBeInTheDocument();
      });

      fireEvent.click(screen.getByTestId('custom-form-submit'));

      await waitFor(() => {
        expect(createCustomProvider).toHaveBeenCalledWith({
          body: {
            engine: 'openai',
            display_name: 'My Custom',
            api_url: 'https://api.example.com',
            api_key: 'key',
            models: ['model-1'],
            supports_streaming: true,
            requires_auth: true,
          },
          throwOnError: true,
        });
      });

      await waitFor(() => {
        expect(mockRefresh).toHaveBeenCalled();
      });
    });

    it('closes modal when cancel is clicked', async () => {
      renderWithProviders(
        <ProviderGrid providers={[]} isOnboarding={false} refreshProviders={mockRefresh} />
      );

      const customCard = screen.getByTestId('add-custom-provider-card');
      const button = customCard.querySelector('button[aria-label="Add a custom provider"]');
      expect(button).toBeTruthy();
      fireEvent.click(button as HTMLElement);

      await waitFor(() => {
        expect(screen.getByTestId('custom-provider-form')).toBeInTheDocument();
      });

      fireEvent.click(screen.getByTestId('custom-form-cancel'));

      await waitFor(() => {
        expect(screen.queryByTestId('custom-provider-form')).not.toBeInTheDocument();
      });
    });

    it('opens SwitchModelModal after custom provider is created', async () => {
      const { createCustomProvider } = await import('@/api');
      (createCustomProvider as Mock).mockResolvedValue({ data: {} });

      renderWithProviders(
        <ProviderGrid providers={[]} isOnboarding={false} refreshProviders={mockRefresh} />
      );

      const customCard = screen.getByTestId('add-custom-provider-card');
      const button = customCard.querySelector('button[aria-label="Add a custom provider"]');
      expect(button).toBeTruthy();
      fireEvent.click(button as HTMLElement);

      await waitFor(() => {
        expect(screen.getByTestId('custom-provider-form')).toBeInTheDocument();
      });

      fireEvent.click(screen.getByTestId('custom-form-submit'));

      await waitFor(() => {
        expect(screen.getByTestId('switch-model-modal')).toBeInTheDocument();
      });
    });
  });

  describe('Regular provider card click (non-onboarding)', () => {
    it('opens ProviderConfigurationModal when a built-in provider card is clicked', async () => {
      const provider = makeProvider();

      renderWithProviders(
        <ProviderGrid providers={[provider]} isOnboarding={false} refreshProviders={mockRefresh} />
      );

      const card = screen.getByTestId('provider-card-openai');
      const button = card.querySelector('button');
      expect(button).toBeTruthy();
      fireEvent.click(button as HTMLElement);

      await waitFor(() => {
        expect(screen.getByTestId('provider-config-modal')).toBeInTheDocument();
        expect(screen.getByTestId('config-modal-provider')).toHaveTextContent('openai');
      });
    });

    it('fetches custom provider details when a Custom provider card is clicked', async () => {
      const { getCustomProvider } = await import('@/api');
      (getCustomProvider as Mock).mockResolvedValue({
        data: {
          config: {
            engine: 'openai',
            display_name: 'My Custom',
            base_url: 'https://api.example.com',
            models: [{ name: 'model-1' }],
            supports_streaming: true,
            requires_auth: true,
          },
          is_editable: true,
        },
      });

      const provider = makeCustomProvider();

      renderWithProviders(
        <ProviderGrid providers={[provider]} isOnboarding={false} refreshProviders={mockRefresh} />
      );

      const card = screen.getByTestId('provider-card-my-custom');
      const button = card.querySelector('button');
      expect(button).toBeTruthy();
      fireEvent.click(button as HTMLElement);

      await waitFor(() => {
        expect(getCustomProvider).toHaveBeenCalledWith({
          path: { id: 'my-custom' },
          throwOnError: true,
        });
      });

      await waitFor(() => {
        expect(screen.getByTestId('custom-provider-form')).toBeInTheDocument();
      });
    });

    it('does NOT call onConfigure when card is clicked during onboarding', () => {
      const provider = makeProvider();

      renderWithProviders(
        <ProviderGrid providers={[provider]} isOnboarding={true} refreshProviders={mockRefresh} />
      );

      // In onboarding, ProviderCard.handleCardClick guards with !isOnboarding
      expect(screen.queryByTestId('provider-config-modal')).not.toBeInTheDocument();
    });
  });

  describe('Provider configuration → model selection flow', () => {
    it('opens SwitchModelModal after provider is configured with correct initial provider', async () => {
      const provider = makeProvider();

      renderWithProviders(
        <ProviderGrid
          providers={[provider]}
          isOnboarding={false}
          refreshProviders={mockRefresh}
          setView={mockSetView}
          onModelSelected={mockOnModelSelected}
        />
      );

      // Click provider card to open config modal
      const card = screen.getByTestId('provider-card-openai');
      const button = card.querySelector('button');
      expect(button).toBeTruthy();
      fireEvent.click(button as HTMLElement);

      await waitFor(() => {
        expect(screen.getByTestId('provider-config-modal')).toBeInTheDocument();
      });

      // Simulate successful configuration
      fireEvent.click(screen.getByTestId('config-modal-configured'));

      await waitFor(() => {
        expect(screen.getByTestId('switch-model-modal')).toBeInTheDocument();
        expect(screen.getByTestId('switch-model-initial-provider')).toHaveTextContent('openai');
      });

      expect(mockRefresh).toHaveBeenCalled();
    });

    it('closes config modal and refreshes when close is clicked', async () => {
      const provider = makeProvider();

      renderWithProviders(
        <ProviderGrid providers={[provider]} isOnboarding={false} refreshProviders={mockRefresh} />
      );

      const card = screen.getByTestId('provider-card-openai');
      const button = card.querySelector('button');
      expect(button).toBeTruthy();
      fireEvent.click(button as HTMLElement);

      await waitFor(() => {
        expect(screen.getByTestId('provider-config-modal')).toBeInTheDocument();
      });

      fireEvent.click(screen.getByTestId('config-modal-close'));

      await waitFor(() => {
        expect(screen.queryByTestId('provider-config-modal')).not.toBeInTheDocument();
      });

      expect(mockRefresh).toHaveBeenCalled();
    });
  });

  describe('edge cases', () => {
    it('works without optional props (setView, onModelSelected)', () => {
      renderWithProviders(<ProviderGrid providers={[makeProvider()]} isOnboarding={false} />);

      expect(screen.getByTestId('provider-card-openai')).toBeInTheDocument();
      expect(screen.getByTestId('add-custom-provider-card')).toBeInTheDocument();
    });
  });
});
