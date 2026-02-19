import '@testing-library/jest-dom';
import { vi, afterEach } from 'vitest';
import { cleanup } from '@testing-library/react';

// Mock Electron modules before any imports
vi.mock('electron', () => ({
  app: {
    getPath: vi.fn((name: string) => {
      if (name === 'userData') return '/tmp/test-user-data';
      if (name === 'temp') return '/tmp';
      if (name === 'home') return '/tmp/home';
      return '/tmp';
    }),
  },
  ipcRenderer: {
    invoke: vi.fn(),
    send: vi.fn(),
    on: vi.fn(),
    off: vi.fn(),
  },
}));

// This is the standard set up to ensure that React Testing Library's
// automatic cleanup runs after each test.
afterEach(() => {
  cleanup();
});

// Mock console methods to avoid noise in tests
// eslint-disable-next-line no-undef
global.console = {
  ...console,
  log: vi.fn(),
  warn: vi.fn(),
  error: vi.fn(),
};

// Mock window.navigator.clipboard for copy functionality tests
Object.assign(navigator, {
  clipboard: {
    writeText: vi.fn(() => Promise.resolve()),
  },
});

// Mock window.electron for renderer process
Object.defineProperty(window, 'electron', {
  writable: true,
  value: {
    platform: 'darwin',
    getSettings: vi.fn(() =>
      Promise.resolve({
        envToggles: {
          GOOSE_SERVER__MEMORY: false,
          GOOSE_SERVER__COMPUTER_CONTROLLER: false,
        },
        showMenuBarIcon: true,
        showDockIcon: true,
        enableWakelock: false,
        spellcheckEnabled: true,
        keyboardShortcuts: {
          focusWindow: 'CommandOrControl+Alt+G',
          quickLauncher: 'CommandOrControl+Alt+Shift+G',
          newChat: 'CommandOrControl+T',
          newChatWindow: 'CommandOrControl+N',
          openDirectory: 'CommandOrControl+O',
          settings: 'CommandOrControl+,',
          find: 'CommandOrControl+F',
          findNext: 'CommandOrControl+G',
          findPrevious: 'CommandOrControl+Shift+G',
          alwaysOnTop: 'CommandOrControl+Shift+T',
        },
      })
    ),
    saveSettings: vi.fn(() => Promise.resolve(true)),
    showMessageBox: vi.fn(() => Promise.resolve({ response: 0 })),
  },
});
