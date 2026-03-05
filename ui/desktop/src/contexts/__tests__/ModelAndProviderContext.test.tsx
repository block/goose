import { act, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ModelAndProviderProvider, useModelAndProvider } from '../ModelAndProviderContext';

// ── API mocks ────────────────────────────────────────────────────────────────
const mockUpdateAgentProvider = vi.fn();
const mockSetConfigProvider = vi.fn();

vi.mock('@/api', () => ({
  updateAgentProvider: (...args: unknown[]) => mockUpdateAgentProvider(...args),
  setConfigProvider: (...args: unknown[]) => mockSetConfigProvider(...args),
}));

// ── Config context mock ──────────────────────────────────────────────────────
const mockRead = vi.fn();
const mockGetProviders = vi.fn();

vi.mock('../ConfigContext', () => ({
  useConfig: () => ({
    read: mockRead,
    getProviders: mockGetProviders,
  }),
}));

vi.mock('@/contexts/ConfigContext', () => ({
  useConfig: () => ({
    read: mockRead,
    getProviders: mockGetProviders,
  }),
}));

// ── Toast mocks ──────────────────────────────────────────────────────────────
const mockToastSuccess = vi.fn();
const mockToastError = vi.fn();

vi.mock('@/toasts', () => ({
  toastSuccess: (...args: unknown[]) => mockToastSuccess(...args),
  toastError: (...args: unknown[]) => mockToastError(...args),
}));

// ── Model display name / provider display name mocks ─────────────────────────
const mockGetModelDisplayName = vi.fn((name: string) => name);
const mockGetProviderDisplayName = vi.fn((_name: string): string | null => null);

vi.mock('@/components/organisms/settings/models/predefinedModelsUtils', () => ({
  getModelDisplayName: (name: string) => mockGetModelDisplayName(name),
  getProviderDisplayName: (name: string) => mockGetProviderDisplayName(name),
}));

// ── getProviderMetadata mock ─────────────────────────────────────────────────
const mockGetProviderMetadata = vi.fn();

vi.mock('@/components/organisms/settings/models/modelInterface', () => ({
  getProviderMetadata: (...args: unknown[]) => mockGetProviderMetadata(...args),
}));

// ── conversionUtils mock ─────────────────────────────────────────────────────
vi.mock('@/utils/conversionUtils', () => ({
  errorMessage: (e: unknown) => (e instanceof Error ? e.message : String(e)),
}));

// ── window.appConfig mock ────────────────────────────────────────────────────
beforeEach(() => {
  (
    window as unknown as {
      appConfig: { get: ReturnType<typeof vi.fn>; set: ReturnType<typeof vi.fn> };
    }
  ).appConfig = {
    get: vi.fn((key: string) => {
      if (key === 'GOOSE_DEFAULT_PROVIDER') return 'openai';
      if (key === 'GOOSE_DEFAULT_MODEL') return 'gpt-4o';
      return undefined;
    }),
  };
});

// ── Test helper: renders the context and returns the hook values ──────────────
function TestConsumer({
  onContext,
}: {
  onContext: (ctx: ReturnType<typeof useModelAndProvider>) => void;
}) {
  const ctx = useModelAndProvider();
  onContext(ctx);
  return (
    <div>
      <span data-testid="model">{ctx.currentModel ?? 'null'}</span>
      <span data-testid="provider">{ctx.currentProvider ?? 'null'}</span>
    </div>
  );
}

function renderWithProvider() {
  let contextRef: ReturnType<typeof useModelAndProvider> | null = null;

  const result = render(
    <ModelAndProviderProvider>
      <TestConsumer
        onContext={(ctx) => {
          contextRef = ctx;
        }}
      />
    </ModelAndProviderProvider>
  );

  return {
    ...result,
    getContext: () => {
      if (!contextRef) throw new Error('Context not yet available');
      return contextRef;
    },
  };
}

// ── Default mock setup ───────────────────────────────────────────────────────
function setupDefaultMocks() {
  mockRead.mockImplementation(async (key: string) => {
    if (key === 'GOOSE_MODEL') return 'gpt-4o';
    if (key === 'GOOSE_PROVIDER') return 'openai';
    return null;
  });

  mockGetProviders.mockResolvedValue([
    {
      is_configured: true,
      name: 'openai',
      metadata: {
        display_name: 'OpenAI',
        known_models: [{ name: 'gpt-4o' }, { name: 'gpt-4o-mini' }],
      },
    },
    {
      is_configured: true,
      name: 'anthropic',
      metadata: {
        display_name: 'Anthropic',
        known_models: [{ name: 'claude-sonnet-4-20250514' }],
      },
    },
  ]);

  mockGetProviderMetadata.mockImplementation(async (providerName: string) => {
    if (providerName === 'openai') {
      return { display_name: 'OpenAI', known_models: [{ name: 'gpt-4o' }] };
    }
    if (providerName === 'anthropic') {
      return { display_name: 'Anthropic', known_models: [{ name: 'claude-sonnet-4-20250514' }] };
    }
    throw new Error(`No match for provider: ${providerName}`);
  });

  mockUpdateAgentProvider.mockResolvedValue({});
  mockSetConfigProvider.mockResolvedValue({});
}

// ═════════════════════════════════════════════════════════════════════════════
// Tests
// ═════════════════════════════════════════════════════════════════════════════

describe('ModelAndProviderContext', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ── Initialization ──────────────────────────────────────────────────────

  describe('initialization', () => {
    it('loads current model and provider from config on mount', async () => {
      setupDefaultMocks();

      renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
        expect(screen.getByTestId('provider')).toHaveTextContent('openai');
      });

      expect(mockRead).toHaveBeenCalledWith('GOOSE_MODEL', false);
      expect(mockRead).toHaveBeenCalledWith('GOOSE_PROVIDER', false);
    });

    it('shows null state when config read fails', async () => {
      mockRead.mockRejectedValue(new Error('Config not found'));
      mockGetProviders.mockResolvedValue([]);

      renderWithProvider();

      // Should remain null since getCurrentModelAndProvider throws
      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('null');
        expect(screen.getByTestId('provider')).toHaveTextContent('null');
      });
    });

    it('falls back when GOOSE_MODEL is missing but GOOSE_PROVIDER exists', async () => {
      mockRead.mockImplementation(async (key: string) => {
        if (key === 'GOOSE_MODEL') return null;
        if (key === 'GOOSE_PROVIDER') return 'openai';
        return null;
      });
      mockGetProviders.mockResolvedValue([]);
      mockSetConfigProvider.mockResolvedValue({});

      renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
        expect(screen.getByTestId('provider')).toHaveTextContent('openai');
      });
    });
  });

  // ── changeModel: happy path ─────────────────────────────────────────────

  describe('changeModel', () => {
    it('updates agent provider, persists config, and updates local state', async () => {
      setupDefaultMocks();

      const { getContext } = renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
      });

      const newModel = {
        name: 'claude-sonnet-4-20250514',
        provider: 'anthropic',
      };

      await act(async () => {
        await getContext().changeModel('session-123', newModel);
      });

      // Verify updateAgentProvider was called with correct payload
      expect(mockUpdateAgentProvider).toHaveBeenCalledWith({
        body: {
          session_id: 'session-123',
          provider: 'anthropic',
          model: 'claude-sonnet-4-20250514',
          context_limit: undefined,
          request_params: undefined,
        },
      });

      // Verify setConfigProvider was called with correct payload
      expect(mockSetConfigProvider).toHaveBeenCalledWith({
        body: {
          provider: 'anthropic',
          model: 'claude-sonnet-4-20250514',
        },
        throwOnError: true,
      });

      // Verify local state was updated
      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('claude-sonnet-4-20250514');
        expect(screen.getByTestId('provider')).toHaveTextContent('anthropic');
      });

      // Verify success toast was shown
      expect(mockToastSuccess).toHaveBeenCalledTimes(1);
      expect(mockToastSuccess).toHaveBeenCalledWith(
        expect.objectContaining({
          msg: expect.stringContaining('claude-sonnet-4-20250514'),
        })
      );
    });

    it('passes context_limit and request_params when provided', async () => {
      setupDefaultMocks();

      const { getContext } = renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
      });

      const newModel = {
        name: 'gemini-3-thinking',
        provider: 'google',
        context_limit: 200000,
        request_params: { thinking_level: 'high' },
      };

      await act(async () => {
        await getContext().changeModel('session-456', newModel);
      });

      expect(mockUpdateAgentProvider).toHaveBeenCalledWith({
        body: {
          session_id: 'session-456',
          provider: 'google',
          model: 'gemini-3-thinking',
          context_limit: 200000,
          request_params: { thinking_level: 'high' },
        },
      });
    });

    it('skips agent update when sessionId is null (no active session)', async () => {
      setupDefaultMocks();

      const { getContext } = renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
      });

      await act(async () => {
        await getContext().changeModel(null, {
          name: 'claude-sonnet-4-20250514',
          provider: 'anthropic',
        });
      });

      // updateAgentProvider should NOT be called when sessionId is null
      expect(mockUpdateAgentProvider).not.toHaveBeenCalled();

      // But config should still be persisted
      expect(mockSetConfigProvider).toHaveBeenCalledWith({
        body: {
          provider: 'anthropic',
          model: 'claude-sonnet-4-20250514',
        },
        throwOnError: true,
      });

      // Local state should be updated
      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('claude-sonnet-4-20250514');
        expect(screen.getByTestId('provider')).toHaveTextContent('anthropic');
      });

      expect(mockToastSuccess).toHaveBeenCalledTimes(1);
    });
  });

  // ── changeModel: failure scenarios ──────────────────────────────────────

  describe('changeModel error handling', () => {
    it('shows error toast when agent update fails and does not persist config', async () => {
      setupDefaultMocks();
      mockUpdateAgentProvider.mockRejectedValueOnce(new Error('Connection refused'));

      const { getContext } = renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
      });

      await act(async () => {
        await getContext().changeModel('session-123', {
          name: 'claude-sonnet-4-20250514',
          provider: 'anthropic',
        });
      });

      // Config should NOT have been called (agent failed first)
      expect(mockSetConfigProvider).not.toHaveBeenCalled();

      // State should NOT be updated
      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
        expect(screen.getByTestId('provider')).toHaveTextContent('openai');
      });

      // Error toast should be shown
      expect(mockToastError).toHaveBeenCalledTimes(1);
      expect(mockToastError).toHaveBeenCalledWith(
        expect.objectContaining({
          title: 'anthropic/claude-sonnet-4-20250514 failed',
        })
      );

      expect(mockToastSuccess).not.toHaveBeenCalled();
    });

    it('when config persist fails after agent update, rolls back agent and shows error toast', async () => {
      setupDefaultMocks();
      mockSetConfigProvider.mockRejectedValueOnce(new Error('Disk full'));

      const { getContext } = renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
      });

      await act(async () => {
        await getContext().changeModel('session-123', {
          name: 'claude-sonnet-4-20250514',
          provider: 'anthropic',
        });
      });

      // Agent was updated once for the new model, then rolled back
      expect(mockUpdateAgentProvider).toHaveBeenCalledTimes(2);
      // Second call should restore the previous model
      expect(mockUpdateAgentProvider).toHaveBeenLastCalledWith(
        expect.objectContaining({
          body: expect.objectContaining({
            session_id: 'session-123',
            provider: 'openai',
            model: 'gpt-4o',
          }),
        })
      );

      // Error toast with rollback message
      expect(mockToastError).toHaveBeenCalledTimes(1);
      expect(mockToastError).toHaveBeenCalledWith(
        expect.objectContaining({
          title: 'anthropic/claude-sonnet-4-20250514 config save failed',
          msg: 'Model change was rolled back',
        })
      );

      // Local state stays on previous model (rolled back)
      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
        expect(screen.getByTestId('provider')).toHaveTextContent('openai');
      });
    });

    it('when config persist fails and rollback also fails, updates state to match agent', async () => {
      setupDefaultMocks();
      mockSetConfigProvider.mockRejectedValueOnce(new Error('Disk full'));
      // First call succeeds (forward), second call fails (rollback)
      mockUpdateAgentProvider
        .mockResolvedValueOnce({})
        .mockRejectedValueOnce(new Error('Agent unreachable'));

      const { getContext } = renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
      });

      await act(async () => {
        await getContext().changeModel('session-123', {
          name: 'claude-sonnet-4-20250514',
          provider: 'anthropic',
        });
      });

      // Both agent calls were made
      expect(mockUpdateAgentProvider).toHaveBeenCalledTimes(2);

      // Error toast mentions rollback failure
      expect(mockToastError).toHaveBeenCalledTimes(1);
      expect(mockToastError).toHaveBeenCalledWith(
        expect.objectContaining({
          title: 'anthropic/claude-sonnet-4-20250514 config save failed',
          msg: 'Model is active for this session but may revert on restart. Rollback also failed.',
        })
      );

      // Local state updated to match what agent is actually running (new model)
      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('claude-sonnet-4-20250514');
        expect(screen.getByTestId('provider')).toHaveTextContent('anthropic');
      });
    });

    it('when config persist fails with no session, shows rolled-back message without agent call', async () => {
      setupDefaultMocks();
      mockSetConfigProvider.mockRejectedValueOnce(new Error('Disk full'));

      const { getContext } = renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
      });

      await act(async () => {
        await getContext().changeModel(null, {
          name: 'claude-sonnet-4-20250514',
          provider: 'anthropic',
        });
      });

      // No agent calls (sessionId is null)
      expect(mockUpdateAgentProvider).not.toHaveBeenCalled();

      // Error toast with rollback message
      expect(mockToastError).toHaveBeenCalledTimes(1);
      expect(mockToastError).toHaveBeenCalledWith(
        expect.objectContaining({
          msg: 'Model change was rolled back',
        })
      );

      // Local state stays on previous model
      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
        expect(screen.getByTestId('provider')).toHaveTextContent('openai');
      });
    });

    it('uses model alias and subtext in success toast when available', async () => {
      setupDefaultMocks();

      const { getContext } = renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
      });

      await act(async () => {
        await getContext().changeModel('session-123', {
          name: 'claude-sonnet-4-20250514',
          provider: 'anthropic',
          alias: 'Claude Sonnet',
          subtext: 'Anthropic',
        });
      });

      expect(mockToastSuccess).toHaveBeenCalledWith(
        expect.objectContaining({
          msg: expect.stringContaining('Claude Sonnet'),
        })
      );
      expect(mockToastSuccess).toHaveBeenCalledWith(
        expect.objectContaining({
          msg: expect.stringContaining('Anthropic'),
        })
      );
    });
  });

  // ── getCurrentModelAndProvider ───────────────────────────────────────────

  describe('getCurrentModelAndProvider', () => {
    it('reads both GOOSE_MODEL and GOOSE_PROVIDER from config', async () => {
      setupDefaultMocks();

      const { getContext } = renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
      });

      const result = await act(async () => {
        return await getContext().getCurrentModelAndProvider();
      });

      expect(result).toEqual({ model: 'gpt-4o', provider: 'openai' });
      expect(mockRead).toHaveBeenCalledWith('GOOSE_MODEL', false);
      expect(mockRead).toHaveBeenCalledWith('GOOSE_PROVIDER', false);
    });

    it('falls back to appConfig defaults when config values are empty', async () => {
      mockRead.mockImplementation(async (key: string) => {
        if (key === 'GOOSE_MODEL') return '';
        if (key === 'GOOSE_PROVIDER') return '';
        return null;
      });
      mockGetProviders.mockResolvedValue([]);
      mockSetConfigProvider.mockResolvedValue({});

      const { getContext } = renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
      });

      const result = await act(async () => {
        return await getContext().getCurrentModelAndProvider();
      });

      expect(result).toEqual({ model: 'gpt-4o', provider: 'openai' });
    });

    it('throws when config read completely fails', async () => {
      mockRead.mockRejectedValue(new Error('Config corrupted'));
      mockGetProviders.mockResolvedValue([]);

      const { getContext } = renderWithProvider();

      // Wait a tick for mount
      await act(async () => {
        await new Promise((r) => setTimeout(r, 50));
      });

      await expect(getContext().getCurrentModelAndProvider()).rejects.toThrow(
        'Failed to read GOOSE_MODEL or GOOSE_PROVIDER from config'
      );
    });
  });

  // ── getFallbackModelAndProvider ─────────────────────────────────────────

  describe('getFallbackModelAndProvider', () => {
    it('returns defaults from window.appConfig', async () => {
      setupDefaultMocks();

      const { getContext } = renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
      });

      const result = await act(async () => {
        return await getContext().getFallbackModelAndProvider();
      });

      expect(result).toEqual({ model: 'gpt-4o', provider: 'openai' });
    });

    it('attempts to persist fallback values to config', async () => {
      setupDefaultMocks();

      const { getContext } = renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
      });

      mockSetConfigProvider.mockClear();

      await act(async () => {
        await getContext().getFallbackModelAndProvider();
      });

      expect(mockSetConfigProvider).toHaveBeenCalledWith({
        body: {
          provider: 'openai',
          model: 'gpt-4o',
        },
        throwOnError: true,
      });
    });
  });

  // ── refreshCurrentModelAndProvider ──────────────────────────────────────

  describe('refreshCurrentModelAndProvider', () => {
    it('re-reads config and updates local state', async () => {
      setupDefaultMocks();

      const { getContext } = renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
        expect(screen.getByTestId('provider')).toHaveTextContent('openai');
      });

      // Change what config returns
      mockRead.mockImplementation(async (key: string) => {
        if (key === 'GOOSE_MODEL') return 'claude-sonnet-4-20250514';
        if (key === 'GOOSE_PROVIDER') return 'anthropic';
        return null;
      });

      await act(async () => {
        await getContext().refreshCurrentModelAndProvider();
      });

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('claude-sonnet-4-20250514');
        expect(screen.getByTestId('provider')).toHaveTextContent('anthropic');
      });
    });

    it('keeps existing state when refresh fails', async () => {
      setupDefaultMocks();

      const { getContext } = renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
      });

      // Make config read fail
      mockRead.mockRejectedValue(new Error('Network error'));

      await act(async () => {
        await getContext().refreshCurrentModelAndProvider();
      });

      // State should remain unchanged
      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
        expect(screen.getByTestId('provider')).toHaveTextContent('openai');
      });
    });
  });

  // ── setProviderAndModel ─────────────────────────────────────────────────

  describe('setProviderAndModel', () => {
    it('directly sets local state without any API calls', async () => {
      setupDefaultMocks();

      const { getContext } = renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
      });

      mockUpdateAgentProvider.mockClear();
      mockSetConfigProvider.mockClear();

      act(() => {
        getContext().setProviderAndModel('anthropic', 'claude-sonnet-4-20250514');
      });

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('claude-sonnet-4-20250514');
        expect(screen.getByTestId('provider')).toHaveTextContent('anthropic');
      });

      // No API calls should have been made
      expect(mockUpdateAgentProvider).not.toHaveBeenCalled();
      expect(mockSetConfigProvider).not.toHaveBeenCalled();
    });
  });

  // ── getCurrentModelAndProviderForDisplay ────────────────────────────────

  describe('getCurrentModelAndProviderForDisplay', () => {
    it('returns provider display name from metadata', async () => {
      setupDefaultMocks();

      const { getContext } = renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
      });

      const result = await act(async () => {
        return await getContext().getCurrentModelAndProviderForDisplay();
      });

      expect(result).toEqual({ model: 'gpt-4o', provider: 'OpenAI' });
    });

    it('falls back to raw provider name when metadata lookup fails', async () => {
      setupDefaultMocks();
      mockGetProviderMetadata.mockRejectedValue(new Error('Provider not found'));

      const { getContext } = renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
      });

      const result = await act(async () => {
        return await getContext().getCurrentModelAndProviderForDisplay();
      });

      expect(result).toEqual({ model: 'gpt-4o', provider: 'openai' });
    });
  });

  // ── getCurrentModelDisplayName ──────────────────────────────────────────

  describe('getCurrentModelDisplayName', () => {
    it('returns display name from predefined models utils', async () => {
      setupDefaultMocks();
      mockGetModelDisplayName.mockReturnValue('GPT-4o');

      const { getContext } = renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
      });

      const displayName = await act(async () => {
        return await getContext().getCurrentModelDisplayName();
      });

      expect(displayName).toBe('GPT-4o');
      expect(mockGetModelDisplayName).toHaveBeenCalledWith('gpt-4o');
    });

    it('returns "Select Model" when config read fails', async () => {
      setupDefaultMocks();

      const { getContext } = renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
      });

      mockRead.mockRejectedValueOnce(new Error('Read failed'));

      const displayName = await act(async () => {
        return await getContext().getCurrentModelDisplayName();
      });

      expect(displayName).toBe('Select Model');
    });
  });

  // ── getCurrentProviderDisplayName ───────────────────────────────────────

  describe('getCurrentProviderDisplayName', () => {
    it('returns predefined provider display name when available', async () => {
      setupDefaultMocks();
      mockGetProviderDisplayName.mockReturnValue('OpenAI');

      const { getContext } = renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
      });

      const displayName = await act(async () => {
        return await getContext().getCurrentProviderDisplayName();
      });

      expect(displayName).toBe('OpenAI');
    });

    it('falls back to metadata lookup when predefined returns null', async () => {
      setupDefaultMocks();
      mockGetProviderDisplayName.mockReturnValue(null);

      const { getContext } = renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
      });

      const displayName = await act(async () => {
        return await getContext().getCurrentProviderDisplayName();
      });

      // Falls back to getCurrentModelAndProviderForDisplay which returns 'OpenAI'
      expect(displayName).toBe('OpenAI');
    });

    it('returns empty string when all lookups fail', async () => {
      setupDefaultMocks();
      mockGetProviderDisplayName.mockReturnValue(null);

      const { getContext } = renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
      });

      mockRead.mockRejectedValueOnce(new Error('Read failed'));

      const displayName = await act(async () => {
        return await getContext().getCurrentProviderDisplayName();
      });

      expect(displayName).toBe('');
    });
  });

  // ── useModelAndProvider hook guard ──────────────────────────────────────

  describe('useModelAndProvider hook', () => {
    it('throws when used outside provider', () => {
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      expect(() => {
        function BadConsumer() {
          useModelAndProvider();
          return null;
        }
        render(<BadConsumer />);
      }).toThrow('useModelAndProvider must be used within a ModelAndProviderProvider');

      consoleSpy.mockRestore();
    });
  });

  // ── isChangingModel guard ─────────────────────────────────────────────

  describe('isChangingModel', () => {
    it('exposes isChangingModel as false initially', async () => {
      setupDefaultMocks();

      const { getContext } = renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
      });

      expect(getContext().isChangingModel).toBe(false);
    });

    it('resets isChangingModel to false after successful change', async () => {
      setupDefaultMocks();

      const { getContext } = renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
      });

      await act(async () => {
        await getContext().changeModel('session-123', {
          name: 'claude-sonnet-4-20250514',
          provider: 'anthropic',
        });
      });

      expect(getContext().isChangingModel).toBe(false);
    });

    it('resets isChangingModel to false after agent update failure', async () => {
      setupDefaultMocks();
      mockUpdateAgentProvider.mockRejectedValueOnce(new Error('Agent down'));

      const { getContext } = renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
      });

      await act(async () => {
        await getContext().changeModel('session-123', {
          name: 'claude-sonnet-4-20250514',
          provider: 'anthropic',
        });
      });

      expect(getContext().isChangingModel).toBe(false);
    });

    it('resets isChangingModel to false after config persist failure with rollback', async () => {
      setupDefaultMocks();
      mockSetConfigProvider.mockRejectedValueOnce(new Error('Disk full'));

      const { getContext } = renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
      });

      await act(async () => {
        await getContext().changeModel('session-123', {
          name: 'claude-sonnet-4-20250514',
          provider: 'anthropic',
        });
      });

      expect(getContext().isChangingModel).toBe(false);
    });
  });

  // ── Sequential model switches ──────────────────────────────────────────

  describe('sequential model switches', () => {
    it('second switch overwrites first switch state', async () => {
      setupDefaultMocks();

      const { getContext } = renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o');
      });

      // First switch
      await act(async () => {
        await getContext().changeModel('session-123', {
          name: 'claude-sonnet-4-20250514',
          provider: 'anthropic',
        });
      });

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('claude-sonnet-4-20250514');
      });

      // Second switch
      await act(async () => {
        await getContext().changeModel('session-123', {
          name: 'gpt-4o-mini',
          provider: 'openai',
        });
      });

      await waitFor(() => {
        expect(screen.getByTestId('model')).toHaveTextContent('gpt-4o-mini');
        expect(screen.getByTestId('provider')).toHaveTextContent('openai');
      });

      // Both API calls should have happened
      expect(mockUpdateAgentProvider).toHaveBeenCalledTimes(2);
      expect(mockSetConfigProvider).toHaveBeenCalledTimes(2);
      expect(mockToastSuccess).toHaveBeenCalledTimes(2);
    });
  });
});
