import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import OnboardingGuard from './OnboardingGuard';

// --- localStorage shim (jsdom in this repo has a broken localStorage) ---

const localStorageStore: Record<string, string> = {};

const localStorageMock = {
  getItem: vi.fn((key: string) => localStorageStore[key] ?? null),
  setItem: vi.fn((key: string, value: string) => {
    localStorageStore[key] = value;
  }),
  removeItem: vi.fn((key: string) => {
    delete localStorageStore[key];
  }),
  clear: vi.fn(() => {
    for (const key of Object.keys(localStorageStore)) {
      delete localStorageStore[key];
    }
  }),
  get length() {
    return Object.keys(localStorageStore).length;
  },
  key: vi.fn((index: number) => Object.keys(localStorageStore)[index] ?? null),
};

Object.defineProperty(globalThis, 'localStorage', {
  value: localStorageMock,
  writable: true,
});

// --- Mocks ---

const mockRead = vi.fn();

vi.mock('../ConfigContext', () => ({
  useConfig: () => ({
    read: mockRead,
    upsert: vi.fn(),
    getProviders: vi.fn().mockResolvedValue([]),
  }),
}));

vi.mock('../ModelAndProviderContext', () => ({
  useModelAndProvider: () => ({
    refreshCurrentModelAndProvider: vi.fn(),
  }),
}));

vi.mock('../../utils/analytics', () => ({
  trackOnboardingStarted: vi.fn(),
  trackOnboardingCompleted: vi.fn(),
  trackOnboardingProviderSelected: vi.fn(),
  trackTelemetryPreference: vi.fn(),
  setTelemetryEnabled: vi.fn(),
}));

// Mock child components that have their own complex dependencies
vi.mock('./ProviderSelector', () => ({
  default: () => <div data-testid="provider-selector">ProviderSelector</div>,
}));

vi.mock('./OnboardingSuccess', () => ({
  default: () => <div data-testid="onboarding-success">OnboardingSuccess</div>,
}));

const renderGuard = () =>
  render(
    <MemoryRouter>
      <OnboardingGuard>
        <div data-testid="app-content">App Content</div>
      </OnboardingGuard>
    </MemoryRouter>
  );

describe('OnboardingGuard', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorageMock.clear();
  });

  it('renders children when provider is configured', async () => {
    mockRead.mockResolvedValue('openai');

    renderGuard();

    await waitFor(() => {
      expect(screen.getByTestId('app-content')).toBeInTheDocument();
    });
  });

  it('sets localStorage flag when provider is detected', async () => {
    mockRead.mockResolvedValue('openai');

    renderGuard();

    await waitFor(() => {
      expect(localStorageMock.setItem).toHaveBeenCalledWith('goose_has_provider', 'true');
    });
  });

  it('shows onboarding when config read fails and no previous provider flag', async () => {
    mockRead.mockRejectedValue(new Error('Connection refused'));

    renderGuard();

    await waitFor(() => {
      expect(screen.getByText('Welcome to goose')).toBeInTheDocument();
    });
    expect(screen.queryByTestId('app-content')).not.toBeInTheDocument();
  });

  it('renders children (not onboarding) when config read fails but user was previously configured', async () => {
    // Simulate an existing user whose goosed is down
    localStorageStore['goose_has_provider'] = 'true';
    mockRead.mockRejectedValue(new Error('Connection refused'));

    renderGuard();

    await waitFor(() => {
      expect(screen.getByTestId('app-content')).toBeInTheDocument();
    });
    expect(screen.queryByText('Welcome to goose')).not.toBeInTheDocument();
  });

  it('shows onboarding for first-time user when provider is empty string', async () => {
    mockRead.mockResolvedValue('');

    renderGuard();

    await waitFor(() => {
      expect(screen.getByText('Welcome to goose')).toBeInTheDocument();
    });
    expect(screen.queryByTestId('app-content')).not.toBeInTheDocument();
  });

  it('does not set localStorage flag when provider is whitespace-only', async () => {
    mockRead.mockResolvedValue('  ');

    renderGuard();

    await waitFor(() => {
      expect(screen.getByText('Welcome to goose')).toBeInTheDocument();
    });
    expect(localStorageMock.setItem).not.toHaveBeenCalled();
  });
});
