import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useExtensionInstallModal } from './useExtensionInstallModal';
import { addExtensionFromDeepLink } from '../components/settings/extensions/deeplink';

// Mock electron APIs
const mockElectron = {
  getConfig: vi.fn(),
  getAllowedExtensions: vi.fn(),
  logInfo: vi.fn(),
  processExtensionLink: vi.fn(),
  on: vi.fn(),
  off: vi.fn(),
};

// Mock the extractExtensionName function
vi.mock('../components/settings/extensions/utils', () => ({
  extractExtensionName: vi.fn((link: string) => {
    const url = new URL(link);
    return url.searchParams.get('name') || 'Unknown Extension';
  }),
}));

// Mock the addExtensionFromDeepLink function
vi.mock('../components/settings/extensions/deeplink', () => ({
  addExtensionFromDeepLink: vi.fn(),
}));

// Set up global window.electron mock
beforeEach(() => {
  Object.defineProperty(globalThis, 'window', {
    value: {
      electron: mockElectron,
    },
    writable: true,
  });

  // Default config - not strict allowlist
  mockElectron.getConfig.mockReturnValue({
    GOOSE_ALLOWLIST_WARNING: true,
  });
});

afterEach(() => {
  vi.clearAllMocks();
});

describe('useExtensionInstallModal', () => {
  const mockAddExtension = vi.fn();

  describe('Initial State', () => {
    it('should initialize with correct default state', () => {
      const { result } = renderHook(() => useExtensionInstallModal(mockAddExtension));

      expect(result.current.modalState).toEqual({
        isOpen: false,
        modalType: 'trusted',
        extensionInfo: null,
        isPending: false,
        error: null,
      });
      expect(result.current.modalConfig).toBeNull();
    });
  });

  describe('Extension Request Handling', () => {
    it('should handle trusted extension (no allowlist)', async () => {
      mockElectron.getAllowedExtensions.mockResolvedValue([]);

      const { result } = renderHook(() => useExtensionInstallModal(mockAddExtension));

      await act(async () => {
        await result.current.handleExtensionRequest(
          'goose://extension?cmd=npx&arg=test-extension&name=TestExt'
        );
      });

      expect(result.current.modalState.isOpen).toBe(true);
      expect(result.current.modalState.modalType).toBe('trusted');
      expect(result.current.modalState.extensionInfo?.name).toBe('TestExt');
      expect(result.current.modalConfig?.title).toBe('Confirm Extension Installation');
    });

    it('should handle untrusted extension (not in allowlist, non-strict)', async () => {
      mockElectron.getAllowedExtensions.mockResolvedValue(['uvx allowed-package']);

      const { result } = renderHook(() => useExtensionInstallModal(mockAddExtension));

      await act(async () => {
        await result.current.handleExtensionRequest(
          'goose://extension?cmd=npx&arg=untrusted-extension&name=UntrustedExt'
        );
      });

      expect(result.current.modalState.modalType).toBe('untrusted');
      expect(result.current.modalConfig?.title).toBe('Install Untrusted Extension?');
      expect(result.current.modalConfig?.confirmLabel).toBe('Install Anyway');
      expect(result.current.modalConfig?.showSingleButton).toBe(false);
    });

    it('should handle blocked extension (not in allowlist, strict mode)', async () => {
      // Set strict allowlist mode
      mockElectron.getConfig.mockReturnValue({
        GOOSE_ALLOWLIST_WARNING: false, // This makes STRICT_ALLOWLIST = true
      });
      mockElectron.getAllowedExtensions.mockResolvedValue(['uvx allowed-package']);

      const { result } = renderHook(() => useExtensionInstallModal(mockAddExtension));

      await act(async () => {
        await result.current.handleExtensionRequest(
          'goose://extension?cmd=npx&arg=blocked-extension&name=BlockedExt'
        );
      });

      expect(result.current.modalState.modalType).toBe('blocked');
      expect(result.current.modalConfig?.title).toBe('Extension Installation Blocked');
      expect(result.current.modalConfig?.confirmLabel).toBe('OK');
      expect(result.current.modalConfig?.showSingleButton).toBe(true);
      expect(result.current.modalConfig?.isBlocked).toBe(true);
    });

    it('should handle remote URL extension as untrusted', async () => {
      const { result } = renderHook(() => useExtensionInstallModal(mockAddExtension));

      await act(async () => {
        await result.current.handleExtensionRequest(
          'goose://extension?type=remote&url=https://api.example.com/mcp&name=RemoteExt'
        );
      });

      expect(result.current.modalState.modalType).toBe('untrusted');
      expect(result.current.modalState.extensionInfo?.remoteUrl).toBe(
        'https://api.example.com/mcp'
      );
      expect(result.current.modalConfig?.message).toContain('connects to a remote service');
    });

    it('should handle extension in allowlist as trusted', async () => {
      mockElectron.getAllowedExtensions.mockResolvedValue(['npx test-extension']);

      const { result } = renderHook(() => useExtensionInstallModal(mockAddExtension));

      await act(async () => {
        await result.current.handleExtensionRequest(
          'goose://extension?cmd=npx&arg=test-extension&name=AllowedExt'
        );
      });

      expect(result.current.modalState.modalType).toBe('trusted');
      expect(result.current.modalConfig?.title).toBe('Confirm Extension Installation');
    });
  });

  describe('Modal Actions', () => {
    it('should dismiss modal correctly', async () => {
      const { result } = renderHook(() => useExtensionInstallModal(mockAddExtension));

      // First open a modal
      await act(async () => {
        await result.current.handleExtensionRequest('goose://extension?cmd=npx&arg=test&name=Test');
      });

      expect(result.current.modalState.isOpen).toBe(true);

      // Then dismiss it
      act(() => {
        result.current.dismissModal();
      });

      expect(result.current.modalState.isOpen).toBe(false);
      expect(result.current.modalState.extensionInfo).toBeNull();
    });

    it('should handle successful extension installation', async () => {
      vi.mocked(addExtensionFromDeepLink).mockResolvedValue(undefined);

      const { result } = renderHook(() => useExtensionInstallModal(mockAddExtension));

      // Open modal with trusted extension
      await act(async () => {
        await result.current.handleExtensionRequest('goose://extension?cmd=npx&arg=test&name=Test');
      });

      // Confirm installation
      let installResult;
      await act(async () => {
        installResult = await result.current.confirmInstall();
      });

      expect(installResult).toEqual({ success: true });
      expect(addExtensionFromDeepLink).toHaveBeenCalledWith(
        'goose://extension?cmd=npx&arg=test&name=Test',
        mockAddExtension,
        expect.any(Function)
      );
      expect(result.current.modalState.isOpen).toBe(false);
    });

    it('should handle failed extension installation', async () => {
      const error = new Error('Installation failed');
      vi.mocked(addExtensionFromDeepLink).mockRejectedValue(error);

      const { result } = renderHook(() => useExtensionInstallModal(mockAddExtension));

      // Open modal
      await act(async () => {
        await result.current.handleExtensionRequest('goose://extension?cmd=npx&arg=test&name=Test');
      });

      // Confirm installation
      let installResult;
      await act(async () => {
        installResult = await result.current.confirmInstall();
      });

      expect(installResult).toEqual({
        success: false,
        error: 'Installation failed',
      });
      expect(result.current.modalState.error).toBe('Installation failed');
    });

    it('should not install blocked extensions', async () => {
      // Set strict mode before creating the hook
      mockElectron.getConfig.mockReturnValue({
        GOOSE_ALLOWLIST_WARNING: false, // This makes STRICT_ALLOWLIST = true
      });
      mockElectron.getAllowedExtensions.mockResolvedValue(['uvx allowed-package']);

      const { result } = renderHook(() => useExtensionInstallModal(mockAddExtension));

      // Open blocked extension modal - use a command not in the allowlist
      await act(async () => {
        await result.current.handleExtensionRequest(
          'goose://extension?cmd=npx&arg=blocked&name=Blocked'
        );
      });

      expect(result.current.modalState.modalType).toBe('blocked');

      // Try to confirm (should fail)
      let installResult;
      await act(async () => {
        installResult = await result.current.confirmInstall();
      });

      expect(installResult).toEqual({
        success: false,
        error: 'No pending extension to install',
      });
      expect(addExtensionFromDeepLink).not.toHaveBeenCalled();
    });
  });

  describe('Error Handling', () => {
    it('should handle electron API errors gracefully', async () => {
      mockElectron.getAllowedExtensions.mockRejectedValue(new Error('API Error'));

      const { result } = renderHook(() => useExtensionInstallModal(mockAddExtension));

      await act(async () => {
        await result.current.handleExtensionRequest('goose://extension?cmd=npx&arg=test&name=Test');
      });

      // Should default to trusted when allowlist check fails
      expect(result.current.modalState.modalType).toBe('trusted');
      expect(result.current.modalState.isOpen).toBe(true);
    });

    it('should handle malformed extension links', async () => {
      const { result } = renderHook(() => useExtensionInstallModal(mockAddExtension));

      await act(async () => {
        await result.current.handleExtensionRequest('invalid-link');
      });

      expect(result.current.modalState.error).toBeTruthy();
      expect(result.current.modalState.isOpen).toBe(false);
    });
  });

  describe('Electron Event Listener', () => {
    it('should set up electron event listener on mount', () => {
      renderHook(() => useExtensionInstallModal());

      expect(mockElectron.on).toHaveBeenCalledWith('add-extension', expect.any(Function));
    });

    it('should clean up electron event listener on unmount', () => {
      const { unmount } = renderHook(() => useExtensionInstallModal());

      unmount();

      expect(mockElectron.off).toHaveBeenCalledWith('add-extension', expect.any(Function));
    });
  });
});
