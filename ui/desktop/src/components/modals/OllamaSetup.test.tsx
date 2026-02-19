import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { toastService } from '../../toasts';
import * as ollamaDetection from '../../utils/ollamaDetection';
import * as providerUtils from '../../utils/providerUtils';
import { OllamaSetup } from './OllamaSetup';

// Mock dependencies
vi.mock('../../utils/ollamaDetection');
vi.mock('../../utils/providerUtils');
vi.mock('../../toasts');

// Mock useConfig hook
const mockUpsert = vi.fn();
const mockAddExtension = vi.fn();
const mockGetExtensions = vi.fn();

vi.mock('../../contexts/ConfigContext', () => ({
  useConfig: () => ({
    upsert: mockUpsert,
    addExtension: mockAddExtension,
    getExtensions: mockGetExtensions,
  }),
}));

describe('OllamaSetup', () => {
  const mockOnSuccess = vi.fn();
  const mockOnCancel = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    // Default mocks
    vi.mocked(ollamaDetection.getPreferredModel).mockReturnValue('gpt-oss:20b');
    vi.mocked(ollamaDetection.getOllamaDownloadUrl).mockReturnValue('https://ollama.com/download');
  });

  describe('when Ollama is not detected', () => {
    beforeEach(() => {
      vi.mocked(ollamaDetection.checkOllamaStatus).mockResolvedValue({
        isRunning: false,
        host: 'http://127.0.0.1:11434',
      });
    });

    it('should show installation instructions', async () => {
      render(<OllamaSetup onSuccess={mockOnSuccess} onCancel={mockOnCancel} />);

      await waitFor(() => {
        expect(screen.getByText('Ollama Setup')).toBeInTheDocument();
        expect(screen.getByText(/Ollama is not detected on your system/)).toBeInTheDocument();
      });
    });

    it('should provide download link', async () => {
      render(<OllamaSetup onSuccess={mockOnSuccess} onCancel={mockOnCancel} />);

      await waitFor(() => {
        const downloadLink = screen.getByRole('link', { name: /Install Ollama/ });
        expect(downloadLink).toHaveAttribute('href', 'https://ollama.com/download');
        expect(downloadLink).toHaveAttribute('target', '_blank');
      });
    });

    it('should show polling state when install link is clicked', async () => {
      render(<OllamaSetup onSuccess={mockOnSuccess} onCancel={mockOnCancel} />);

      // Mock pollForOllama
      const mockStopPolling = vi.fn();
      vi.mocked(ollamaDetection.pollForOllama).mockReturnValue(mockStopPolling);

      await waitFor(() => {
        const installLink = screen.getByText('Install Ollama');
        fireEvent.click(installLink);
      });

      expect(screen.getByText(/Waiting for Ollama to start/)).toBeInTheDocument();
      expect(ollamaDetection.pollForOllama).toHaveBeenCalled();
    });

    it('should handle cancel button', async () => {
      render(<OllamaSetup onSuccess={mockOnSuccess} onCancel={mockOnCancel} />);

      await waitFor(() => {
        fireEvent.click(screen.getByText('Cancel'));
      });

      expect(mockOnCancel).toHaveBeenCalled();
    });
  });

  describe('when Ollama is detected but model is not available', () => {
    beforeEach(() => {
      vi.mocked(ollamaDetection.checkOllamaStatus).mockResolvedValue({
        isRunning: true,
        host: 'http://127.0.0.1:11434',
      });
      vi.mocked(ollamaDetection.hasModel).mockResolvedValue(false);
    });

    it('should show model download prompt', async () => {
      render(<OllamaSetup onSuccess={mockOnSuccess} onCancel={mockOnCancel} />);

      await waitFor(() => {
        expect(screen.getByText(/The gpt-oss:20b model is not installed/)).toBeInTheDocument();
        expect(screen.getByText(/Download gpt-oss:20b/)).toBeInTheDocument();
      });
    });

    it('should handle model download', async () => {
      vi.mocked(ollamaDetection.pullOllamaModel).mockResolvedValue(true);

      render(<OllamaSetup onSuccess={mockOnSuccess} onCancel={mockOnCancel} />);

      await waitFor(() => {
        fireEvent.click(screen.getByText(/Download gpt-oss:20b/));
      });

      await waitFor(() => {
        expect(toastService.success).toHaveBeenCalledWith(
          expect.objectContaining({
            title: 'Model Downloaded!',
          })
        );
      });
    });

    it('should handle download failure', async () => {
      vi.mocked(ollamaDetection.pullOllamaModel).mockResolvedValue(false);

      render(<OllamaSetup onSuccess={mockOnSuccess} onCancel={mockOnCancel} />);

      await waitFor(() => {
        fireEvent.click(screen.getByText(/Download gpt-oss:20b/));
      });

      await waitFor(() => {
        expect(toastService.error).toHaveBeenCalledWith(
          expect.objectContaining({
            title: 'Download Failed',
          })
        );
      });
    });
  });

  // TODO: re-enable when we have ollama back in the onboarding
  describe.skip('when Ollama and model are both available', () => {
    beforeEach(() => {
      vi.mocked(ollamaDetection.checkOllamaStatus).mockResolvedValue({
        isRunning: true,
        host: 'http://127.0.0.1:11434',
      });
      vi.mocked(ollamaDetection.hasModel).mockResolvedValue(true);
    });

    it('should show ready state and connect button', async () => {
      render(<OllamaSetup onSuccess={mockOnSuccess} onCancel={mockOnCancel} />);

      await waitFor(() => {
        expect(screen.getByText(/Ollama is detected and running/)).toBeInTheDocument();
      });
    });

    it('should handle successful connection', async () => {
      vi.mocked(providerUtils.initializeSystem).mockResolvedValue(undefined);

      render(<OllamaSetup onSuccess={mockOnSuccess} onCancel={mockOnCancel} />);

      await waitFor(() => {
        fireEvent.click(screen.getByText(/Use Goose with Ollama/));
      });

      await waitFor(() => {
        expect(mockUpsert).toHaveBeenCalledWith('GOOSE_PROVIDER', 'ollama', false);
        expect(mockUpsert).toHaveBeenCalledWith('GOOSE_MODEL', 'gpt-oss:20b', false);
        expect(mockUpsert).toHaveBeenCalledWith('OLLAMA_HOST', 'localhost', false);
        expect(providerUtils.initializeSystem).toHaveBeenCalledWith(
          'ollama',
          'gpt-oss:20b',
          expect.any(Object)
        );
        expect(toastService.success).toHaveBeenCalled();
        expect(mockOnSuccess).toHaveBeenCalled();
      });
    });

    it('should handle connection failure', async () => {
      const testError = new Error('Initialization failed');
      vi.mocked(providerUtils.initializeSystem).mockRejectedValue(testError);

      render(<OllamaSetup onSuccess={mockOnSuccess} onCancel={mockOnCancel} />);

      await waitFor(() => {
        fireEvent.click(screen.getByText('Use Goose with Ollama'));
      });

      await waitFor(() => {
        expect(toastService.error).toHaveBeenCalledWith(
          expect.objectContaining({
            title: 'Connection Failed',
            msg: expect.stringContaining('Initialization failed'),
          })
        );
      });
    });
  });

  describe('polling behavior', () => {
    it('should clean up polling on unmount', async () => {
      const mockStopPolling = vi.fn();
      vi.mocked(ollamaDetection.pollForOllama).mockReturnValue(mockStopPolling);

      vi.mocked(ollamaDetection.checkOllamaStatus).mockResolvedValue({
        isRunning: false,
        host: 'http://127.0.0.1:11434',
      });

      const { unmount } = render(<OllamaSetup onSuccess={mockOnSuccess} onCancel={mockOnCancel} />);

      await waitFor(() => {
        fireEvent.click(screen.getByText('Install Ollama'));
      });

      expect(ollamaDetection.pollForOllama).toHaveBeenCalled();

      unmount();

      expect(mockStopPolling).toHaveBeenCalled();
    });

    it('should handle Ollama detection during polling', async () => {
      vi.mocked(ollamaDetection.checkOllamaStatus).mockResolvedValue({
        isRunning: false,
        host: 'http://127.0.0.1:11434',
      });

      let pollCallback: ((status: { isRunning: boolean; host: string }) => void) | undefined;
      vi.mocked(ollamaDetection.pollForOllama).mockImplementation((onDetected) => {
        pollCallback = onDetected;
        return vi.fn();
      });

      render(<OllamaSetup onSuccess={mockOnSuccess} onCancel={mockOnCancel} />);

      await waitFor(() => {
        fireEvent.click(screen.getByText('Install Ollama'));
      });

      expect(screen.getByText(/Waiting for Ollama/)).toBeInTheDocument();

      // Simulate Ollama being detected
      vi.mocked(ollamaDetection.hasModel).mockResolvedValue(true);
      pollCallback?.({ isRunning: true, host: 'http://127.0.0.1:11434' });

      await waitFor(() => {
        expect(screen.getByText('Ollama is detected and running')).toBeInTheDocument();
      });
    });
  });

  describe('error states', () => {
    it('should handle errors during initial check', async () => {
      // Mock checkOllamaStatus to resolve with isRunning: false after an error
      vi.mocked(ollamaDetection.checkOllamaStatus).mockResolvedValue({
        isRunning: false,
        host: 'http://127.0.0.1:11434',
        error: 'Network error',
      });

      render(<OllamaSetup onSuccess={mockOnSuccess} onCancel={mockOnCancel} />);

      await waitFor(() => {
        // Should still show not detected state
        expect(screen.getByText('Ollama is not detected on your system')).toBeInTheDocument();
      });
    });
  });
});
