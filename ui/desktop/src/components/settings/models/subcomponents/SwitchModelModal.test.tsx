import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, type RenderOptions } from '@testing-library/react';
import type { ReactElement } from 'react';
import { SwitchModelModal } from './SwitchModelModal';
import { IntlTestWrapper } from '../../../../i18n/test-utils';
import { setCuratedModels } from '../predefinedModelsUtils';

const acpListProviderDetails = vi.fn();
const acpListProviderModels = vi.fn();

vi.mock('../../../../acp/providers', () => ({
  acpListProviderDetails: (...args: unknown[]) => acpListProviderDetails(...args),
  acpListProviderModels: (...args: unknown[]) => acpListProviderModels(...args),
  acpReadThinkingEffort: vi.fn().mockResolvedValue(null),
  acpSaveThinkingEffort: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('../../../ModelAndProviderContext', () => ({
  useModelAndProvider: () => ({
    changeModel: vi.fn().mockResolvedValue(true),
    currentModel: 'zeta/first',
    currentProvider: 'avocado',
  }),
}));

vi.mock('../../../../utils/analytics', () => ({
  trackModelChanged: vi.fn(),
}));

vi.mock('../modelInterface', async () => {
  const actual = await vi.importActual('../modelInterface');
  return {
    ...actual,
    fetchModelReasoning: vi.fn().mockResolvedValue(false),
    fetchModelsForProviders: vi.fn().mockResolvedValue([]),
    getProviderMetadata: vi.fn(),
  };
});

const renderWithIntl = (ui: ReactElement, options?: RenderOptions) =>
  render(ui, { wrapper: IntlTestWrapper, ...options });

describe('SwitchModelModal inventory catalog', () => {
  beforeEach(() => {
    setCuratedModels([]);
    vi.stubGlobal('appConfig', {
      get: vi.fn((key: string) => {
        if (key === 'GOOSE_PREDEFINED_MODELS') return '';
        return undefined;
      }),
    });
    acpListProviderDetails.mockResolvedValue([
      {
        name: 'avocado',
        is_configured: true,
        provider_type: 'Builtin',
        metadata: {
          name: 'avocado',
          display_name: 'Avocado',
          description: 'Sign in',
          default_model: 'zeta/first',
          model_doc_link: '',
          config_keys: [],
          known_models: [],
        },
      },
    ]);
    acpListProviderModels.mockResolvedValue([
      {
        id: 'zeta/first',
        name: 'Zeta First',
        alias: 'Zeta First',
        subtext: 'Should stay first',
        contextLimit: null,
        reasoning: null,
        recommended: true,
      },
      {
        id: 'alpha/second',
        name: 'Zeta Model QA',
        alias: 'Zeta Model QA',
        subtext: 'Distinct alias for anti-hardcode',
        contextLimit: null,
        reasoning: null,
        recommended: true,
      },
    ]);
  });

  it('GivenAvocadoInventoryWithAliasSubtext_WhenRenderingPicker_ThenShowsAliasNotRawId', async () => {
    // covers AC-4
    renderWithIntl(
      <SwitchModelModal sessionId={null} onClose={vi.fn()} setView={vi.fn()} />
    );

    await waitFor(() => {
      expect(screen.getByText('Zeta First')).toBeInTheDocument();
    });
    expect(screen.getByText('Zeta Model QA')).toBeInTheDocument();
    expect(screen.getByText('Should stay first')).toBeInTheDocument();
    expect(screen.queryByText('zeta/first')).not.toBeInTheDocument();
  });
});
