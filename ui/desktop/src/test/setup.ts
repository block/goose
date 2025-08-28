import '@testing-library/jest-dom';
import { vi, afterEach } from 'vitest';
import { cleanup } from '@testing-library/react';

// This is the standard setup to ensure that React Testing Library's
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
