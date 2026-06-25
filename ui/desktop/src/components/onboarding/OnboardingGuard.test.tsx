import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, type RenderOptions } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import OnboardingGuard from './OnboardingGuard';
import { IntlTestWrapper } from '../../i18n/test-utils';

const mockNavigate = vi.fn();
const mockRead = vi.fn();
const mockGetFallbackModelAndProvider = vi.fn();
const mockRefreshCurrentModelAndProvider = vi.fn();

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return {
    ...actual,
    useNavigate: () => mockNavigate,
  };
});

vi.mock('../ConfigContext', () => ({
  useConfig: () => ({
    read: mockRead,
    upsert: vi.fn(),
    getProviders: vi.fn(),
  }),
}));

vi.mock('./ProviderSelector', () => ({
  default: () => null,
}));

vi.mock('./OnboardingSuccess', () => ({
  default: () => null,
}));

vi.mock('../ModelAndProviderContext', () => ({
  useModelAndProvider: () => ({
    getFallbackModelAndProvider: mockGetFallbackModelAndProvider,
    refreshCurrentModelAndProvider: mockRefreshCurrentModelAndProvider,
  }),
}));

vi.mock('../../utils/analytics', () => ({
  trackOnboardingStarted: vi.fn(),
  trackOnboardingCompleted: vi.fn(),
  trackOnboardingProviderSelected: vi.fn(),
  trackTelemetryPreference: vi.fn(),
  setTelemetryEnabled: vi.fn(),
}));

const renderWithProviders = (
  ui: React.ReactElement,
  { route = '/' }: { route?: string } = {},
  options?: RenderOptions
) =>
  render(
    <IntlTestWrapper>
      <MemoryRouter initialEntries={[route]}>
        <Routes>
          <Route path="*" element={ui} />
        </Routes>
      </MemoryRouter>
    </IntlTestWrapper>,
    options
  );

describe('OnboardingGuard', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockRead.mockRejectedValue(new Error('backend unavailable'));
    mockGetFallbackModelAndProvider.mockResolvedValue({ provider: '', model: '' });
    mockRefreshCurrentModelAndProvider.mockResolvedValue(undefined);
  });

  it(
    'shows recovery actions when the backend check fails',
    async () => {
      renderWithProviders(
        <OnboardingGuard>
          <div>child content</div>
        </OnboardingGuard>
      );

      expect(await screen.findByText('Unable to connect to Goose server', {}, { timeout: 8000 })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'Open Settings' })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'Retry' })).toBeInTheDocument();
    },
    10000
  );

  it(
    'navigates to Goose Server settings when Open Settings is clicked',
    async () => {
      const user = userEvent.setup();
      renderWithProviders(
        <OnboardingGuard>
          <div>child content</div>
        </OnboardingGuard>
      );

      await screen.findByText('Unable to connect to Goose server', {}, { timeout: 8000 });
      await user.click(screen.getByRole('button', { name: 'Open Settings' }));

      expect(mockNavigate).toHaveBeenCalledWith('/settings?section=sharing');
    },
    10000
  );

  it(
    'renders settings route children even when the backend check fails',
    async () => {
      renderWithProviders(
        <OnboardingGuard>
          <div>settings content</div>
        </OnboardingGuard>,
        { route: '/settings' }
      );

      await waitFor(
        () => {
          expect(screen.getByText('settings content')).toBeInTheDocument();
        },
        { timeout: 8000 }
      );
      expect(screen.queryByText('Unable to connect to Goose server')).not.toBeInTheDocument();
    },
    10000
  );
});
