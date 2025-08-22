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
    _remoteUrl: string | null
  ): Promise<ModalType> => {
    try {
      const config = window.electron.getConfig();
      const ALLOWLIST_WARNING_MODE = config.GOOSE_ALLOWLIST_WARNING === true;

      // If warning mode is enabled, always show warning but allow installation
      if (ALLOWLIST_WARNING_MODE) {
        return 'untrusted';
      }

      const allowedCommands = await window.electron.getAllowedExtensions();

      // If no allowlist configured
      if (!allowedCommands || allowedCommands.length === 0) {
        return 'trusted';
      }

      const isCommandAllowed = allowedCommands.some((allowedCmd: string) =>
        command.startsWith(allowedCmd)
      );

      return isCommandAllowed ? 'trusted' : 'blocked';
    } catch (error) {
      console.error('Error checking allowlist:', error);
      return 'trusted';
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
          message: `\n\nThis extension command is not in the allowed list and its installation is blocked.\n\nExtension: ${name}\nCommand: ${command || remoteUrl}\n\nContact your administrator to request approval for this extension.`,
          confirmLabel: 'OK',
          cancelLabel: '',
          showSingleButton: true,
          isBlocked: true,
        };

      case 'untrusted': {
        const securityMessage = `\n\nThis extension command is not in the allowed list and will be able to access your conversations and provide additional functionality.\n\nInstalling extensions from untrusted sources may pose security risks.`;

        return {
          title: 'Install Untrusted Extension?',
          message: `${securityMessage}\n\nExtension: ${name}\n${remoteUrl ? `URL: ${remoteUrl}` : `Command: ${command}`}\n\nContact your administrator if you are unsure about this.`,
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

      dismissModal();

      if (addExtension) {
        await addExtensionFromDeepLink(pendingLink, addExtension, () => {
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
