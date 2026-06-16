/* eslint-disable @typescript-eslint/no-explicit-any */

/**
 * @vitest-environment jsdom
 */
import React from 'react';
import { act, screen, render, waitFor } from '@testing-library/react';
import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest';
import { AppInner, getNewChatSourceSessionId, resolveSessionInitialMessage } from './App';
import { IntlTestWrapper } from './i18n/test-utils';
import { getSession } from './api';
import { AppEvents } from './constants/events';

// Set up globals for jsdom
Object.defineProperty(window, 'location', {
  value: {
    hash: '',
    search: '',
    href: 'http://localhost:3000',
    origin: 'http://localhost:3000',
    pathname: '/',
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

vi.mock('./utils/costDatabase', () => ({
  initializeCostDatabase: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('./api', () => {
  const test_chat = {
    data: {
      session_id: 'test',
      messages: [],
      metadata: {
        description: '',
      },
    },
  };

  return {
    initConfig: vi.fn().mockResolvedValue(undefined),
    readAllConfig: vi.fn().mockResolvedValue(undefined),
    backupConfig: vi.fn().mockResolvedValue(undefined),
    recoverConfig: vi.fn().mockResolvedValue(undefined),
    validateConfig: vi.fn().mockResolvedValue(undefined),
    startAgent: vi.fn().mockResolvedValue(test_chat),
    resumeAgent: vi.fn().mockResolvedValue(test_chat),
    getSession: vi.fn(),
  };
});

vi.mock('./sessions', () => ({
  fetchSessionDetails: vi
    .fn()
    .mockResolvedValue({ sessionId: 'test', messages: [], metadata: { description: '' } }),
  generateSessionId: vi.fn(),
  createSession: vi.fn(),
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

vi.mock('./components/ModelAndProviderContext', () => ({
  ModelAndProviderProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  useModelAndProvider: () => ({
    provider: null,
    model: null,
    getCurrentModelAndProvider: vi.fn(),
    getFallbackModelAndProvider: vi.fn().mockResolvedValue({ provider: '', model: '' }),
    refreshCurrentModelAndProvider: vi.fn().mockResolvedValue(undefined),
    setCurrentModelAndProvider: vi.fn(),
  }),
}));

vi.mock('./contexts/ChatContext', () => ({
  ChatProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  useChatContext: () => ({
    chat: {
      id: 'test-id',
      name: 'Test Chat',
      messages: [],
      recipe: null,
    },
    setChat: vi.fn(),
    setPairChat: vi.fn(), // Keep this from HEAD
    resetChat: vi.fn(),
    hasActiveSession: false,
    setRecipe: vi.fn(),
    clearRecipe: vi.fn(),
    contextKey: 'hub',
  }),
  DEFAULT_CHAT_TITLE: 'New Chat', // Keep this from HEAD
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

// Create mocks that we can track and configure per test
const mockNavigate = vi.fn();
const mockSearchParams = new URLSearchParams();
const mockSetSearchParams = vi.fn();

// Mock react-router-dom to avoid HashRouter issues in tests
vi.mock('react-router-dom', () => ({
  HashRouter: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  Routes: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  Route: ({ element }: { element: React.ReactNode }) => element,
  useNavigate: () => mockNavigate,
  useLocation: () => ({ state: null, pathname: '/' }),
  useSearchParams: () => [mockSearchParams, mockSetSearchParams],
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
  getSetting: vi.fn().mockResolvedValue(null),
  setSetting: vi.fn().mockResolvedValue(undefined),
};

const getLatestElectronHandler = (channel: string) => {
  const calls = mockElectron.on.mock.calls.filter(
    ([registeredChannel]) => registeredChannel === channel
  );
  return calls[calls.length - 1]?.[1];
};

// Mock appConfig
const mockAppConfig = {
  get: vi.fn((key: string): string | null => {
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
    mockNavigate.mockClear();
    mockSetSearchParams.mockClear();
    vi.mocked(getSession).mockResolvedValue({
      data: {
        id: 'active-session',
        working_dir: '/active/project',
      },
    } as any);
    mockAppConfig.get.mockImplementation((key: string): string | null => {
      if (key === 'GOOSE_WORKING_DIR') return '/test/dir';
      return null;
    });

    // Reset search params
    mockSearchParams.forEach((_, key) => {
      mockSearchParams.delete(key);
    });

    window.location.hash = '';
    window.location.search = '';
    window.location.pathname = '/';
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

    render(<AppInner />, { wrapper: IntlTestWrapper });

    // Wait for initialization
    await waitFor(() => {
      expect(mockElectron.reactReady).toHaveBeenCalled();
    });

    // The app should initialize without any navigation calls since we're already at "/"
    // No navigate calls should be made when no provider is configured
    expect(mockNavigate).not.toHaveBeenCalled();
  });

  it('should handle deep links correctly when app is brand new', async () => {
    // Mock no provider configured
    mockElectron.getConfig.mockReturnValue({
      GOOSE_DEFAULT_PROVIDER: null,
      GOOSE_DEFAULT_MODEL: null,
      GOOSE_ALLOWLIST_WARNING: false,
    });

    // Set up search params to simulate view=settings deep link
    mockSearchParams.set('view', 'settings');

    render(<AppInner />, { wrapper: IntlTestWrapper });

    // Wait for initialization
    await waitFor(() => {
      expect(mockElectron.reactReady).toHaveBeenCalled();
    });

    expect(screen.getByText(/^Welcome to goose/)).toBeInTheDocument();
  });

  it('should not redirect when provider is configured', async () => {
    // Mock provider configured
    mockElectron.getConfig.mockReturnValue({
      GOOSE_DEFAULT_PROVIDER: 'openai',
      GOOSE_DEFAULT_MODEL: 'gpt-4',
      GOOSE_ALLOWLIST_WARNING: false,
    });

    render(<AppInner />, { wrapper: IntlTestWrapper });

    // Wait for initialization
    await waitFor(() => {
      expect(mockElectron.reactReady).toHaveBeenCalled();
    });

    // Should not navigate anywhere since provider is configured and we're already at "/"
    expect(mockNavigate).not.toHaveBeenCalled();
  });

  it('should navigate home when the main process emits new-chat', async () => {
    mockElectron.getConfig.mockReturnValue({
      GOOSE_DEFAULT_PROVIDER: 'openai',
      GOOSE_DEFAULT_MODEL: 'gpt-4',
      GOOSE_ALLOWLIST_WARNING: false,
    });

    render(<AppInner />, { wrapper: IntlTestWrapper });

    await waitFor(() => {
      expect(mockElectron.reactReady).toHaveBeenCalled();
    });

    const newChatHandler = mockElectron.on.mock.calls.find(([channel]) => channel === 'new-chat')?.[1];
    expect(newChatHandler).toBeDefined();

    newChatHandler?.({} as any);

    expect(mockNavigate).toHaveBeenCalledWith('/', { state: { workingDir: '/test/dir' } });
  });

  it('should use the active session working directory when set-view opens a new chat', async () => {
    mockElectron.getConfig.mockReturnValue({
      GOOSE_DEFAULT_PROVIDER: 'openai',
      GOOSE_DEFAULT_MODEL: 'gpt-4',
      GOOSE_ALLOWLIST_WARNING: false,
    });

    render(<AppInner />, { wrapper: IntlTestWrapper });

    await waitFor(() => {
      expect(mockElectron.reactReady).toHaveBeenCalled();
    });

    const initialSetViewHandlers = mockElectron.on.mock.calls.filter(
      ([channel]) => channel === 'set-view'
    ).length;

    act(() => {
      window.dispatchEvent(
        new CustomEvent(AppEvents.ADD_ACTIVE_SESSION, {
          detail: { sessionId: 'active-session' },
        })
      );
    });

    await waitFor(() => {
      expect(mockElectron.on.mock.calls.filter(([channel]) => channel === 'set-view')).toHaveLength(
        initialSetViewHandlers + 1
      );
    });

    const setViewHandler = getLatestElectronHandler('set-view');
    expect(setViewHandler).toBeDefined();

    act(() => {
      setViewHandler?.({} as any, '');
    });

    await waitFor(() => {
      expect(getSession).toHaveBeenCalledWith({
        path: { session_id: 'active-session' },
        throwOnError: false,
      });
      expect(mockNavigate).toHaveBeenCalledWith('/', {
        state: { workingDir: '/active/project' },
      });
    });
  });

  it('should seed recipe sessions with the recipe prompt when no initial message is provided', () => {
    expect(
      resolveSessionInitialMessage(
        {
          recipe: {
            prompt: 'Write a release note for the latest change',
          },
        },
        undefined
      )
    ).toEqual({
      msg: 'Write a release note for the latest change',
      images: [],
    });
  });

  it('should prefer the visible session when choosing a New Chat source session', () => {
    expect(
      getNewChatSourceSessionId(
        'visible',
        [{ sessionId: 'background' }, { sessionId: 'stale-active' }],
        'chat'
      )
    ).toBe('visible');
  });

  it('should fall back to the last active session before the shared chat session', () => {
    expect(
      getNewChatSourceSessionId(
        undefined,
        [{ sessionId: 'background' }, { sessionId: 'active' }],
        'chat'
      )
    ).toBe('active');
  });

  it('should fall back to the shared chat session when there is no routed or active session', () => {
    expect(getNewChatSourceSessionId(undefined, [], 'chat')).toBe('chat');
  });
});
