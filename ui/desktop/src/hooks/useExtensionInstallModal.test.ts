import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useExtensionInstallModal } from './useExtensionInstallModal';
import { addExtensionFromDeepLink } from '../components/settings/extensions/deeplink';

const mockElectron = {
  getConfig: vi.fn(),
  getAllowedExtensions: vi.fn(),
  logInfo: vi.fn(),
  processExtensionLink: vi.fn(),
  on: vi.fn(),
  off: vi.fn(),
};

vi.mock('../components/settings/extensions/utils', () => ({
  extractExtensionName: vi.fn((link: string) => {
    const url = new URL(link);
    return url.searchParams.get('name') || 'Unknown Extension';
  }),
}));

vi.mock('../components/settings/extensions/deeplink', () => ({
  addExtensionFromDeepLink: vi.fn(),
}));

beforeEach(() => {
  Object.defineProperty(globalThis, 'window', {
    value: {
      electron: mockElectron,
    },
    writable: true,
  });

  mockElectron.getConfig.mockReturnValue({
    GOOSE_ALLOWLIST_WARNING: false,
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
    it('should handle trusted extension (default behaviour, no allowlist)', async () => {
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

    it('should handle trusted extension (from allowlist)', async () => {
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

    it('should handle warning mode', async () => {
      mockElectron.getConfig.mockReturnValue({
        GOOSE_ALLOWLIST_WARNING: true,
      });

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

    it('should handle blocked extension', async () => {
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
  });

  describe('Modal Actions', () => {
    it('should dismiss modal correctly', async () => {
      const { result } = renderHook(() => useExtensionInstallModal(mockAddExtension));

      await act(async () => {
        await result.current.handleExtensionRequest('goose://extension?cmd=npx&arg=test&name=Test');
      });

      expect(result.current.modalState.isOpen).toBe(true);

      act(() => {
        result.current.dismissModal();
      });

      expect(result.current.modalState.isOpen).toBe(false);
      expect(result.current.modalState.extensionInfo).toBeNull();
    });

    it('should handle successful extension installation', async () => {
      vi.mocked(addExtensionFromDeepLink).mockResolvedValue(undefined);
      mockElectron.getAllowedExtensions.mockResolvedValue([]);

      const { result } = renderHook(() => useExtensionInstallModal(mockAddExtension));

      await act(async () => {
        await result.current.handleExtensionRequest('goose://extension?cmd=npx&arg=test&name=Test');
      });

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
      mockElectron.getAllowedExtensions.mockResolvedValue([]);

      const { result } = renderHook(() => useExtensionInstallModal(mockAddExtension));

      await act(async () => {
        await result.current.handleExtensionRequest('goose://extension?cmd=npx&arg=test&name=Test');
      });

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
      mockElectron.getAllowedExtensions.mockResolvedValue(['uvx allowed-package']);

      const { result } = renderHook(() => useExtensionInstallModal(mockAddExtension));

      await act(async () => {
        await result.current.handleExtensionRequest(
          'goose://extension?cmd=npx&arg=blocked&name=Blocked'
        );
      });

      expect(result.current.modalState.modalType).toBe('blocked');

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
});
