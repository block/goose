import { useState, useCallback, useEffect } from 'react';
import { IpcRendererEvent } from 'electron';
import { extractExtensionName } from '../components/settings/extensions/utils';
import { addExtensionFromDeepLink } from '../components/settings/extensions/deeplink';
import type { ExtensionConfig } from '../api/types.gen';
import {
  ExtensionModalState,
  ExtensionInfo,
  ModalType,
  ExtensionModalConfig,
  ExtensionInstallResult,
} from '../types/extension';

// Helper functions extracted from App.tsx
function extractCommand(link: string): string {
  const url = new URL(link);
  const cmd = url.searchParams.get('cmd') || 'Unknown Command';
  const args = url.searchParams.getAll('arg').map(decodeURIComponent);
  return `${cmd} ${args.join(' ')}`.trim();
}

function extractRemoteUrl(link: string): string | null {
  const url = new URL(link);
  return url.searchParams.get('url');
}

export const useExtensionInstallModal = (
  addExtension?: (name: string, config: ExtensionConfig, enabled: boolean) => Promise<void>
) => {
  const [modalState, setModalState] = useState<ExtensionModalState>({
    isOpen: false,
    modalType: 'trusted',
    extensionInfo: null,
    isPending: false,
    error: null,
  });

  const [pendingLink, setPendingLink] = useState<string | null>(null);

  const determineModalType = async (
    command: string,
    remoteUrl: string | null
  ): Promise<ModalType> => {
    // Remote URLs are always treated as untrusted (for now)
    if (remoteUrl) {
      return 'untrusted';
    }

    try {
      const config = window.electron.getConfig();
      const STRICT_ALLOWLIST = config.GOOSE_ALLOWLIST_WARNING !== true;

      const allowedCommands = await window.electron.getAllowedExtensions();
      if (allowedCommands && allowedCommands.length > 0) {
        const isCommandAllowed = allowedCommands.some((allowedCmd: string) =>
          command.startsWith(allowedCmd)
        );

        if (!isCommandAllowed) {
          return STRICT_ALLOWLIST ? 'blocked' : 'untrusted';
        }
      }
      return 'trusted';
    } catch (error) {
      console.error('Error checking allowlist:', error);
      return 'trusted'; // Default to trusted if we can't check
    }
  };

  const generateModalConfig = (
    modalType: ModalType,
    extensionInfo: ExtensionInfo
  ): ExtensionModalConfig => {
    const { name, command, remoteUrl } = extensionInfo;

    switch (modalType) {
      case 'blocked':
        return {
          title: 'Extension Installation Blocked',
          message: `This extension cannot be installed because it is not on your organization's approved list.\n\nExtension: ${name}\nCommand: ${command || remoteUrl}\n\nContact your administrator to request approval for this extension.`,
          confirmLabel: 'OK',
          cancelLabel: '',
          showSingleButton: true,
          isBlocked: true,
        };

      case 'untrusted': {
        const securityMessage = remoteUrl
          ? `This extension connects to a remote service and is not on your organization's approved list.`
          : `This extension is not on your organization's approved list and may pose security risks.`;

        return {
          title: 'Install Untrusted Extension?',
          message: `${securityMessage}\n\nExtension: ${name}\n${remoteUrl ? `URL: ${remoteUrl}` : `Command: ${command}`}\n\nThis extension will be able to access your conversations and provide additional functionality.\n\nOnly install if you trust the source. Contact your administrator if unsure.`,
          confirmLabel: 'Install Anyway',
          cancelLabel: 'Cancel',
          showSingleButton: false,
          isBlocked: false,
        };
      }

      case 'trusted':
      default:
        return {
          title: 'Confirm Extension Installation',
          message: `Are you sure you want to install the ${name} extension?\n\nCommand: ${command || remoteUrl}`,
          confirmLabel: 'Yes',
          cancelLabel: 'No',
          showSingleButton: false,
          isBlocked: false,
        };
    }
  };

  const handleExtensionRequest = useCallback(async (link: string): Promise<void> => {
    try {
      console.log(`Processing extension request: ${link}`);

      const command = extractCommand(link);
      const remoteUrl = extractRemoteUrl(link);
      const extName = extractExtensionName(link);

      const extensionInfo: ExtensionInfo = {
        name: extName,
        command: command,
        remoteUrl: remoteUrl || undefined,
        link: link,
      };

      const modalType = await determineModalType(command, remoteUrl);

      setModalState({
        isOpen: true,
        modalType,
        extensionInfo,
        isPending: false,
        error: null,
      });

      // Set pending link for installation (null if blocked)
      setPendingLink(modalType === 'blocked' ? null : link);

      window.electron.logInfo(`Extension modal opened: ${modalType} for ${extName}`);
    } catch (error) {
      console.error('Error processing extension request:', error);
      setModalState((prev) => ({
        ...prev,
        error: error instanceof Error ? error.message : 'Unknown error',
      }));
    }
  }, []);

  const dismissModal = useCallback(() => {
    setModalState({
      isOpen: false,
      modalType: 'trusted',
      extensionInfo: null,
      isPending: false,
      error: null,
    });
    setPendingLink(null);
  }, []);

  const confirmInstall = useCallback(async (): Promise<ExtensionInstallResult> => {
    if (!pendingLink) {
      return { success: false, error: 'No pending extension to install' };
    }

    setModalState((prev) => ({ ...prev, isPending: true }));

    try {
      console.log(`Confirming installation of extension from: ${pendingLink}`);

      // Close modal immediately
      dismissModal();

      // Process the extension installation using existing deep link handler
      if (addExtension) {
        await addExtensionFromDeepLink(pendingLink, addExtension, () => {
          // Navigation callback - for now just log
          console.log('Extension installation completed, navigating to extensions');
        });
      } else {
        throw new Error('addExtension function not provided to hook');
      }

      return { success: true };
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Installation failed';
      console.error('Extension installation failed:', error);

      setModalState((prev) => ({
        ...prev,
        error: errorMessage,
        isPending: false,
      }));

      return { success: false, error: errorMessage };
    }
  }, [pendingLink, dismissModal, addExtension]);

  const getModalConfig = (): ExtensionModalConfig | null => {
    if (!modalState.extensionInfo) return null;
    return generateModalConfig(modalState.modalType, modalState.extensionInfo);
  };

  // Set up electron event listener
  useEffect(() => {
    console.log('Setting up extension install modal handler');

    const handleAddExtension = async (_event: IpcRendererEvent, ...args: unknown[]) => {
      const link = args[0] as string;
      await handleExtensionRequest(link);
    };

    window.electron.on('add-extension', handleAddExtension);

    return () => {
      window.electron.off('add-extension', handleAddExtension);
    };
  }, [handleExtensionRequest]);

  return {
    modalState,
    modalConfig: getModalConfig(),
    handleExtensionRequest,
    dismissModal,
    confirmInstall,
  };
};
