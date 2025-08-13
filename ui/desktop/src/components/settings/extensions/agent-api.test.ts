import { describe, it, expect, vi, beforeEach } from 'vitest';
import { extensionApiCall, addToAgent, removeFromAgent, sanitizeName } from './agent-api';
import * as config from '../../../config';
import * as toasts from '../../../toasts';
import { ExtensionConfig } from '../../../api/types.gen';

// Mock dependencies
vi.mock('../../../config');
vi.mock('../../../toasts');
vi.mock('./utils');

const mockGetApiUrl = vi.mocked(config.getApiUrl);
const mockToastService = vi.mocked(toasts.toastService);

// Mock window.electron
const mockElectron = {
  getSecretKey: vi.fn(),
};

Object.defineProperty(window, 'electron', {
  value: mockElectron,
  writable: true,
});

// Mock fetch
const mockFetch = vi.fn();
(globalThis as typeof globalThis & { fetch: typeof mockFetch }).fetch = mockFetch;

describe('Agent API', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGetApiUrl.mockImplementation((path: string) => `http://localhost:8080${path}`);
    mockElectron.getSecretKey.mockResolvedValue('secret-key');
    mockToastService.configure = vi.fn();
    mockToastService.loading = vi.fn().mockReturnValue('toast-id');
    mockToastService.success = vi.fn();
    mockToastService.error = vi.fn();
    mockToastService.dismiss = vi.fn();
  });

  describe('sanitizeName', () => {
    it('should sanitize extension names correctly', () => {
      expect(sanitizeName('Test Extension')).toBe('testextension');
      expect(sanitizeName('My-Extension_Name')).toBe('myextensionname');
      expect(sanitizeName('UPPERCASE')).toBe('uppercase');
    });
  });

  describe('extensionApiCall', () => {
    const mockExtensionConfig: ExtensionConfig = {
      type: 'stdio',
      name: 'test-extension',
      cmd: 'python',
      args: ['script.py'],
    };

    it('should make successful API call for adding extension', async () => {
      const mockResponse = {
        ok: true,
        text: vi.fn().mockResolvedValue('{"error": false}'),
      };
      mockFetch.mockResolvedValue(mockResponse);

      const response = await extensionApiCall('/extensions/add', mockExtensionConfig);

      expect(mockFetch).toHaveBeenCalledWith('http://localhost:8080/extensions/add', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': 'secret-key',
        },
        body: JSON.stringify(mockExtensionConfig),
      });

      expect(mockToastService.loading).toHaveBeenCalledWith({
        title: 'test-extension',
        msg: 'Activating test-extension extension...',
      });

      expect(mockToastService.success).toHaveBeenCalledWith({
        title: 'test-extension',
        msg: 'Successfully activated extension',
      });

      expect(response).toBe(mockResponse);
    });

    it('should make successful API call for removing extension', async () => {
      const mockResponse = {
        ok: true,
        text: vi.fn().mockResolvedValue('{"error": false}'),
      };
      mockFetch.mockResolvedValue(mockResponse);

      const response = await extensionApiCall('/extensions/remove', 'test-extension');

      expect(mockFetch).toHaveBeenCalledWith('http://localhost:8080/extensions/remove', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': 'secret-key',
        },
        body: JSON.stringify('test-extension'),
      });

      expect(mockToastService.loading).not.toHaveBeenCalled(); // No loading toast for removal
      expect(mockToastService.success).toHaveBeenCalledWith({
        title: 'test-extension',
        msg: 'Successfully deactivated extension',
      });

      expect(response).toBe(mockResponse);
    });

    it('should handle HTTP error responses', async () => {
      const mockResponse = {
        ok: false,
        status: 500,
        statusText: 'Internal Server Error',
      };
      mockFetch.mockResolvedValue(mockResponse);

      await expect(extensionApiCall('/extensions/add', mockExtensionConfig)).rejects.toThrow(
        'Server returned 500: Internal Server Error'
      );

      expect(mockToastService.error).toHaveBeenCalledWith({
        title: 'test-extension',
        msg: 'Failed to add test-extension extension: Server returned 500: Internal Server Error',
        traceback: 'Server returned 500: Internal Server Error',
      });
    });

    it('should handle 428 error specially', async () => {
      const mockResponse = {
        ok: false,
        status: 428,
        statusText: 'Precondition Required',
      };
      mockFetch.mockResolvedValue(mockResponse);

      await expect(extensionApiCall('/extensions/add', mockExtensionConfig)).rejects.toThrow(
        'Agent is not initialized. Please initialize the agent first.'
      );

      expect(mockToastService.error).toHaveBeenCalledWith({
        title: 'test-extension',
        msg: 'Failed to add extension. Goose Agent was still starting up. Please try again.',
        traceback: 'Server returned 428: Precondition Required',
      });
    });

    it('should handle API error responses', async () => {
      const mockResponse = {
        ok: true,
        text: vi.fn().mockResolvedValue('{"error": true, "message": "Extension not found"}'),
      };
      mockFetch.mockResolvedValue(mockResponse);

      await expect(extensionApiCall('/extensions/remove', 'test-extension')).rejects.toThrow(
        'Error deactivating extension: Extension not found'
      );

      expect(mockToastService.error).toHaveBeenCalledWith({
        title: 'test-extension',
        msg: 'Error deactivating extension: Extension not found',
        traceback: 'Error deactivating extension: Extension not found',
      });
    });

    it('should handle JSON parse errors', async () => {
      const mockResponse = {
        ok: true,
        text: vi.fn().mockResolvedValue('invalid json'),
      };
      mockFetch.mockResolvedValue(mockResponse);

      const response = await extensionApiCall('/extensions/add', mockExtensionConfig);

      expect(mockToastService.success).toHaveBeenCalledWith({
        title: 'test-extension',
        msg: 'Successfully activated extension',
      });

      expect(response).toBe(mockResponse);
    });

    it('should handle network errors', async () => {
      const networkError = new Error('Network error');
      mockFetch.mockRejectedValue(networkError);

      await expect(extensionApiCall('/extensions/add', mockExtensionConfig)).rejects.toThrow(
        'Network error'
      );

      expect(mockToastService.error).toHaveBeenCalledWith({
        title: 'test-extension',
        msg: 'Network error',
        traceback: 'Network error',
      });
    });

    it('should configure toast service with options', async () => {
      const mockResponse = {
        ok: true,
        text: vi.fn().mockResolvedValue('{"error": false}'),
      };
      mockFetch.mockResolvedValue(mockResponse);

      await extensionApiCall('/extensions/add', mockExtensionConfig, { silent: true });

      expect(mockToastService.configure).toHaveBeenCalledWith({ silent: true });
    });
  });

  describe('addToAgent', () => {
    const mockExtensionConfig: ExtensionConfig = {
      type: 'stdio',
      name: 'Test Extension',
      cmd: 'python',
      args: ['script.py'],
    };

    it('should add stdio extension to agent with shim replacement', async () => {
      const mockResponse = {
        ok: true,
        text: vi.fn().mockResolvedValue('{"error": false}'),
      };
      mockFetch.mockResolvedValue(mockResponse);

      // Mock replaceWithShims
      const { replaceWithShims } = await import('./utils');
      vi.mocked(replaceWithShims).mockResolvedValue('/path/to/python');

      await addToAgent(mockExtensionConfig);

      expect(mockFetch).toHaveBeenCalledWith('http://localhost:8080/extensions/add', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': 'secret-key',
        },
        body: JSON.stringify({
          ...mockExtensionConfig,
          name: 'testextension',
          cmd: '/path/to/python',
        }),
      });
    });

    it('should handle 428 error with enhanced message', async () => {
      const mockResponse = {
        ok: false,
        status: 428,
        statusText: 'Precondition Required',
      };
      mockFetch.mockResolvedValue(mockResponse);

      await expect(addToAgent(mockExtensionConfig)).rejects.toThrow(
        'Agent is not initialized. Please initialize the agent first.'
      );
    });

    it('should add non-stdio extension without shim replacement', async () => {
      const sseConfig: ExtensionConfig = {
        type: 'sse',
        name: 'SSE Extension',
        uri: 'http://localhost:8080/events',
      };

      const mockResponse = {
        ok: true,
        text: vi.fn().mockResolvedValue('{"error": false}'),
      };
      mockFetch.mockResolvedValue(mockResponse);

      await addToAgent(sseConfig);

      expect(mockFetch).toHaveBeenCalledWith('http://localhost:8080/extensions/add', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': 'secret-key',
        },
        body: JSON.stringify({
          ...sseConfig,
          name: 'sseextension',
        }),
      });
    });
  });

  describe('removeFromAgent', () => {
    it('should remove extension from agent', async () => {
      const mockResponse = {
        ok: true,
        text: vi.fn().mockResolvedValue('{"error": false}'),
      };
      mockFetch.mockResolvedValue(mockResponse);

      await removeFromAgent('Test Extension');

      expect(mockFetch).toHaveBeenCalledWith('http://localhost:8080/extensions/remove', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': 'secret-key',
        },
        body: JSON.stringify('testextension'),
      });
    });

    it('should handle removal errors', async () => {
      const mockResponse = {
        ok: false,
        status: 404,
        statusText: 'Not Found',
      };
      mockFetch.mockResolvedValue(mockResponse);

      await expect(removeFromAgent('Test Extension')).rejects.toThrow();

      expect(mockToastService.error).toHaveBeenCalled();
    });
  });
});
