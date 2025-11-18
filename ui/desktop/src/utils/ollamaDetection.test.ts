/* eslint-disable @typescript-eslint/no-explicit-any */
/* global AbortSignal, TextEncoder, EventListener */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  checkOllamaStatus,
  getOllamaModels,
  hasModel,
  pullOllamaModel,
  pollForOllama,
  getOllamaDownloadUrl,
  getPreferredModel,
} from './ollamaDetection';

// Mock fetch globally
globalThis.fetch = vi.fn();

// Define global objects for testing environment if they don't exist
if (typeof globalThis.AbortSignal === 'undefined') {
  globalThis.AbortSignal = class AbortSignal {
    aborted = false;
    reason: any = undefined;
    onabort: ((this: AbortSignal, ev: Event) => any) | null = null;

    addEventListener(_type: string, _listener: EventListener): void {
      // Mock implementation
    }

    removeEventListener(_type: string, _listener: EventListener): void {
      // Mock implementation
    }

    dispatchEvent(_event: Event): boolean {
      return true;
    }
  } as any;
}

if (typeof globalThis.TextEncoder === 'undefined') {
  globalThis.TextEncoder = class TextEncoder {
    encode(str: string): Uint8Array {
      return new Uint8Array(str.split('').map((c) => c.charCodeAt(0)));
    }
  } as any;
}

describe('ollamaDetection', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe('checkOllamaStatus', () => {
    it('should return isRunning: true when Ollama is accessible', async () => {
      (globalThis.fetch as any).mockResolvedValueOnce({
        ok: true,
        json: async () => ({ models: [] }),
      });

      const result = await checkOllamaStatus();

      expect(result).toEqual({
        isRunning: true,
        host: 'http://127.0.0.1:11434',
      });
      expect(globalThis.fetch).toHaveBeenCalledWith('http://127.0.0.1:11434/api/tags', {
        method: 'GET',
        signal: expect.any(globalThis.AbortSignal),
      });
    });

    it('should return isRunning: false when Ollama is not accessible', async () => {
      (globalThis.fetch as any).mockRejectedValueOnce(new Error('Connection refused'));

      const result = await checkOllamaStatus();

      expect(result).toEqual({
        isRunning: false,
        host: 'http://127.0.0.1:11434',
        error: 'Connection refused',
      });
    });

    it('should timeout after 2 seconds', async () => {
      let abortSignal: AbortSignal | undefined;

      (globalThis.fetch as any).mockImplementationOnce((_url: string, options: any) => {
        abortSignal = options.signal;
        return new Promise((_, reject) => {
          // Listen for abort signal
          options.signal.addEventListener('abort', () => {
            reject(new Error('The operation was aborted'));
          });
        });
      });

      const checkPromise = checkOllamaStatus();

      // Fast-forward 2 seconds
      vi.advanceTimersByTime(2000);

      // The abort signal should be triggered
      expect(abortSignal?.aborted).toBe(true);

      const result = await checkPromise;
      expect(result.isRunning).toBe(false);
      expect(result.error).toBe('The operation was aborted');
    });
  });

  describe('getOllamaModels', () => {
    it('should return models when API call is successful', async () => {
      const mockModels = [
        { name: 'llama2:latest', size: 4733363377, digest: 'abc123', modified_at: '2023-10-01' },
        { name: 'gpt-oss:20b', size: 13780173839, digest: 'def456', modified_at: '2023-10-02' },
      ];

      (globalThis.fetch as any).mockResolvedValueOnce({
        ok: true,
        json: async () => ({ models: mockModels }),
      });

      const result = await getOllamaModels();

      expect(result).toEqual(mockModels);
    });

    it('should return empty array when API call fails', async () => {
      (globalThis.fetch as any).mockRejectedValueOnce(new Error('Network error'));

      const result = await getOllamaModels();

      expect(result).toEqual([]);
    });

    it('should handle non-ok responses', async () => {
      (globalThis.fetch as any).mockResolvedValueOnce({
        ok: false,
        statusText: 'Not Found',
      });

      const result = await getOllamaModels();

      expect(result).toEqual([]);
    });
  });

  describe('hasModel', () => {
    it('should return true when model exists', async () => {
      (globalThis.fetch as any).mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          models: [
            {
              name: 'llama2:latest',
              size: 4733363377,
              digest: 'abc123',
              modified_at: '2023-10-01',
            },
            { name: 'gpt-oss:20b', size: 13780173839, digest: 'def456', modified_at: '2023-10-02' },
          ],
        }),
      });

      const result = await hasModel('gpt-oss:20b');

      expect(result).toBe(true);
    });

    it('should return false when model does not exist', async () => {
      (globalThis.fetch as any).mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          models: [
            {
              name: 'llama2:latest',
              size: 4733363377,
              digest: 'abc123',
              modified_at: '2023-10-01',
            },
          ],
        }),
      });

      const result = await hasModel('gpt-oss:20b');

      expect(result).toBe(false);
    });
  });

  describe('pullOllamaModel', () => {
    it('should successfully pull a model and report progress', async () => {
      const progressUpdates: any[] = [];
      const onProgress = vi.fn((progress) => progressUpdates.push(progress));

      const mockResponse = {
        ok: true,
        body: {
          getReader: () => ({
            read: vi
              .fn()
              .mockResolvedValueOnce({
                done: false,
                value: new TextEncoder().encode(
                  JSON.stringify({ status: 'downloading', completed: 100, total: 1000 }) + '\n'
                ),
              })
              .mockResolvedValueOnce({
                done: false,
                value: new TextEncoder().encode(
                  JSON.stringify({ status: 'downloading', completed: 500, total: 1000 }) + '\n'
                ),
              })
              .mockResolvedValueOnce({
                done: false,
                value: new TextEncoder().encode(JSON.stringify({ status: 'success' }) + '\n'),
              })
              .mockResolvedValueOnce({ done: true }),
          }),
        },
      };

      (globalThis.fetch as any).mockResolvedValueOnce(mockResponse);

      const result = await pullOllamaModel('gpt-oss:20b', onProgress);

      expect(result).toBe(true);
      expect(onProgress).toHaveBeenCalledTimes(3);
      expect(progressUpdates).toContainEqual({
        status: 'downloading',
        completed: 100,
        total: 1000,
      });
      expect(progressUpdates).toContainEqual({
        status: 'downloading',
        completed: 500,
        total: 1000,
      });
      expect(progressUpdates).toContainEqual({ status: 'success' });
    });

    it('should return false on API error', async () => {
      (globalThis.fetch as any).mockResolvedValueOnce({
        ok: false,
        statusText: 'Model not found',
      });

      const result = await pullOllamaModel('invalid-model');

      expect(result).toBe(false);
    });
  });

  describe('pollForOllama', () => {
    it('should poll until Ollama is detected', async () => {
      const onDetected = vi.fn();

      // First call: Ollama not running
      (globalThis.fetch as any).mockRejectedValueOnce(new Error('Connection refused'));

      const stopPolling = pollForOllama(onDetected, 100);

      // Verify initial call was made
      expect(globalThis.fetch).toHaveBeenCalledTimes(1);

      // Second call: Still not running
      (globalThis.fetch as any).mockRejectedValueOnce(new Error('Connection refused'));
      vi.advanceTimersByTime(100);

      // Third call: Ollama is running
      (globalThis.fetch as any).mockResolvedValueOnce({
        ok: true,
        json: async () => ({ models: [] }),
      });
      vi.advanceTimersByTime(100);

      // Wait for async operations
      await vi.runAllTimersAsync();

      expect(onDetected).toHaveBeenCalledWith({
        isRunning: true,
        host: 'http://127.0.0.1:11434',
      });

      stopPolling();
    });

    it('should stop polling when stop function is called', () => {
      const onDetected = vi.fn();

      (globalThis.fetch as any).mockRejectedValue(new Error('Connection refused'));

      const stopPolling = pollForOllama(onDetected, 100);

      // Should make initial call
      expect(globalThis.fetch).toHaveBeenCalledTimes(1);

      // Stop polling
      stopPolling();

      // Advance time and verify no more calls are made
      vi.advanceTimersByTime(500);

      // Only the initial call should have been made
      expect(globalThis.fetch).toHaveBeenCalledTimes(1);
      expect(onDetected).not.toHaveBeenCalled();
    });
  });

  describe('utility functions', () => {
    it('should return correct download URL', () => {
      expect(getOllamaDownloadUrl()).toBe('https://ollama.com/download');
    });

    it('should return correct preferred model', () => {
      expect(getPreferredModel()).toBe('gpt-oss:20b');
    });
  });
});
