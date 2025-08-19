/* eslint-disable @typescript-eslint/no-explicit-any */

/**
 * @vitest-environment jsdom
 */
import React from 'react';
import { render, waitFor } from '@testing-library/react';
import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest';
import App from './App';

// Set up globals for jsdom
Object.defineProperty(window, 'location', {
  value: {
    hash: '',
    search: '',
    href: 'http://localhost:3000',
    origin: 'http://localhost:3000',
  },
  writable: true,
});

Object.defineProperty(window, 'history', {
  value: {
    replaceState: vi.fn(),
    state: null,
  },
  writable: true,
});

// Mock dependencies
vi.mock('./utils/providerUtils', () => ({
  initializeSystem: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('./utils/costDatabase', () => ({
  initializeCostDatabase: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('./api/sdk.gen', () => ({
  initConfig: vi.fn().mockResolvedValue(undefined),
  readAllConfig: vi.fn().mockResolvedValue(undefined),
  backupConfig: vi.fn().mockResolvedValue(undefined),
  recoverConfig: vi.fn().mockResolvedValue(undefined),
  validateConfig: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('./utils/openRouterSetup', () => ({
  startOpenRouterSetup: vi.fn().mockResolvedValue({ success: false, message: 'Test' }),
}));

vi.mock('./utils/ollamaDetection', () => ({
  checkOllamaStatus: vi.fn().mockResolvedValue({ isRunning: false }),
}));

// Mock the ConfigContext module
vi.mock('./components/ConfigContext', () => ({
  useConfig: () => ({
    read: vi.fn().mockResolvedValue(null),
    update: vi.fn(),
    getExtensions: vi.fn().mockReturnValue([]),
    addExtension: vi.fn(),
    updateExtension: vi.fn(),
    createProviderDefaults: vi.fn(),
  }),
  ConfigProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

// Mock other components to simplify testing
vi.mock('./components/ErrorBoundary', () => ({
  ErrorUI: ({ error }: { error: Error }) => <div>Error: {error.message}</div>,
}));

// Mock ProviderGuard to show the welcome screen when no provider is configured
vi.mock('./components/ProviderGuard', () => ({
  default: ({ children }: { children: React.ReactNode }) => {
    // In a real app, ProviderGuard would check for provider and show welcome screen
    // For this test, we'll simulate that behavior
    const hasProvider = window.electron?.getConfig()?.GOOSE_DEFAULT_PROVIDER;
    if (!hasProvider) {
      return <div>Welcome to Goose!</div>;
    }
    return <>{children}</>;
  },
}));

vi.mock('./components/ModelAndProviderContext', () => ({
  ModelAndProviderProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  useModelAndProvider: () => ({
    provider: null,
    model: null,
    getCurrentModelAndProvider: vi.fn(),
    setCurrentModelAndProvider: vi.fn(),
  }),
}));

vi.mock('./contexts/ChatContext', () => ({
  ChatProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  useChatContext: () => ({
    chat: {
      id: 'test-id',
      title: 'Test Chat',
      messages: [],
      messageHistoryIndex: 0,
      recipeConfig: null,
    },
    setChat: vi.fn(),
    setPairChat: vi.fn(), // Keep this from HEAD
    resetChat: vi.fn(),
    hasActiveSession: false,
    setRecipeConfig: vi.fn(),
    clearRecipeConfig: vi.fn(),
    setRecipeParameters: vi.fn(),
    clearRecipeParameters: vi.fn(),
    draft: '',
    setDraft: vi.fn(),
    clearDraft: vi.fn(),
    contextKey: 'hub',
  }),
  DEFAULT_CHAT_TITLE: 'New Chat', // Keep this from HEAD
}));

vi.mock('./contexts/DraftContext', () => ({
  DraftProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock('./components/ui/ConfirmationModal', () => ({
  ConfirmationModal: () => null,
}));

vi.mock('react-toastify', () => ({
  ToastContainer: () => null,
}));

vi.mock('./components/GoosehintsModal', () => ({
  GoosehintsModal: () => null,
}));

vi.mock('./components/AnnouncementModal', () => ({
  default: () => null,
}));

// Mock react-router-dom to avoid HashRouter issues in tests
vi.mock('react-router-dom', () => ({
  HashRouter: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  Routes: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  Route: ({ element }: { element: React.ReactNode }) => element,
  useNavigate: () => vi.fn(),
  useLocation: () => ({ state: null, pathname: '/' }),
  Outlet: () => null,
}));

// Mock electron API
const mockElectron = {
  getConfig: vi.fn().mockReturnValue({
    GOOSE_ALLOWLIST_WARNING: false,
    GOOSE_WORKING_DIR: '/test/dir',
  }),
  logInfo: vi.fn(),
  on: vi.fn(),
  off: vi.fn(),
  reactReady: vi.fn(),
  getAllowedExtensions: vi.fn().mockResolvedValue([]),
  platform: 'darwin',
  createChatWindow: vi.fn(),
};

// Mock appConfig
const mockAppConfig = {
  get: vi.fn((key: string) => {
    if (key === 'GOOSE_WORKING_DIR') return '/test/dir';
    return null;
  }),
};

// Attach mocks to window
(window as any).electron = mockElectron;
(window as any).appConfig = mockAppConfig;

// Mock matchMedia
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation((query) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(), // deprecated
    removeListener: vi.fn(), // deprecated
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

describe('App Component - Brand New State', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.location.hash = '';
    window.location.search = '';
    window.sessionStorage.clear();
    window.localStorage.clear();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('should redirect to "/" when app is brand new (no provider configured)', async () => {
    // Mock no provider configured
    mockElectron.getConfig.mockReturnValue({
      GOOSE_DEFAULT_PROVIDER: null,
      GOOSE_DEFAULT_MODEL: null,
      GOOSE_ALLOWLIST_WARNING: false,
    });

    render(<App />);

    // Wait for initialization
    await waitFor(() => {
      expect(mockElectron.reactReady).toHaveBeenCalled();
    });

    // Check that we navigated to "/" not "/welcome"
    await waitFor(() => {
      // In some environments, the hash might be empty or just "#"
      expect(window.location.hash).toMatch(/^(#\/?|)$/);
    });

    // History should have been updated to "/"
    expect(window.history.replaceState).toHaveBeenCalledWith({}, '', '#/');
  });

  it('should handle deep links correctly when app is brand new', async () => {
    // Mock no provider configured
    mockElectron.getConfig.mockReturnValue({
      GOOSE_DEFAULT_PROVIDER: null,
      GOOSE_DEFAULT_MODEL: null,
      GOOSE_ALLOWLIST_WARNING: false,
    });

    // Simulate a deep link
    window.location.search = '?view=settings';

    render(<App />);

    // Wait for initialization
    await waitFor(() => {
      expect(mockElectron.reactReady).toHaveBeenCalled();
    });

    // Should redirect to settings route via hash
    await waitFor(() => {
      expect(window.location.hash).toBe('#/settings');
    });
  });

  it('should not redirect to /welcome when provider is configured', async () => {
    // Mock provider configured
    mockElectron.getConfig.mockReturnValue({
      GOOSE_DEFAULT_PROVIDER: 'openai',
      GOOSE_DEFAULT_MODEL: 'gpt-4',
      GOOSE_ALLOWLIST_WARNING: false,
    });

    render(<App />);

    // Wait for initialization
    await waitFor(() => {
      expect(mockElectron.reactReady).toHaveBeenCalled();
    });

    // Should stay at "/" since provider is configured
    await waitFor(() => {
      // In some environments, the hash might be empty or just "#"
      expect(window.location.hash).toMatch(/^(#\/?|)$/);
    });
  });

  it('should handle config recovery gracefully', async () => {
    // Mock config error that triggers recovery
    const { readAllConfig, recoverConfig } = await import('./api/sdk.gen');
    console.log(recoverConfig);
    vi.mocked(readAllConfig).mockRejectedValueOnce(new Error('Config read error'));

    mockElectron.getConfig.mockReturnValue({
      GOOSE_DEFAULT_PROVIDER: null,
      GOOSE_DEFAULT_MODEL: null,
      GOOSE_ALLOWLIST_WARNING: false,
    });

    render(<App />);

    // Wait for initialization and recovery
    await waitFor(() => {
      expect(mockElectron.reactReady).toHaveBeenCalled();
    });

    // App should still initialize and navigate to "/"
    await waitFor(() => {
      // In some environments, the hash might be empty or just "#"
      expect(window.location.hash).toMatch(/^(#\/?|)$/);
    });
  });
});
