import { useState, useCallback, useEffect } from 'react';
import { IpcRendererEvent } from 'electron';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from './ui/dialog';
import { Button } from './ui/button';
import { extractExtensionName } from './settings/extensions/utils';
import { addExtensionFromDeepLink } from './settings/extensions/deeplink';
import type { ExtensionConfig } from '../api/types.gen';

type ModalType = 'blocked' | 'untrusted' | 'trusted';

interface ExtensionInfo {
  name: string;
  command?: string;
  remoteUrl?: string;
  link: string;
}

interface ExtensionModalState {
  isOpen: boolean;
  modalType: ModalType;
  extensionInfo: ExtensionInfo | null;
  isPending: boolean;
  error: string | null;
}

interface ExtensionModalConfig {
  title: string;
  message: string;
  confirmLabel: string;
  cancelLabel: string;
  showSingleButton: boolean;
  isBlocked: boolean;
}

interface ExtensionInstallModalProps {
  addExtension?: (name: string, config: ExtensionConfig, enabled: boolean) => Promise<void>;
}

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

export function ExtensionInstallModal({ addExtension }: ExtensionInstallModalProps) {
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

      if (ALLOWLIST_WARNING_MODE) {
        return 'untrusted';
      }

      const allowedCommands = await window.electron.getAllowedExtensions();

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

  const confirmInstall = useCallback(async (): Promise<void> => {
    if (!pendingLink) {
      return;
    }

    setModalState((prev) => ({ ...prev, isPending: true }));

    try {
      console.log(`Confirming installation of extension from: ${pendingLink}`);

      if (addExtension) {
        await addExtensionFromDeepLink(pendingLink, addExtension, () => {
          console.log('Extension installation completed, navigating to extensions');
        });
      } else {
        throw new Error('addExtension function not provided to component');
      }

      // Only dismiss modal after successful installation
      dismissModal();
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Installation failed';
      console.error('Extension installation failed:', error);

      setModalState((prev) => ({
        ...prev,
        error: errorMessage,
        isPending: false,
      }));
    }
  }, [pendingLink, dismissModal, addExtension]);

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

  const getModalConfig = (): ExtensionModalConfig | null => {
    if (!modalState.extensionInfo) return null;
    return generateModalConfig(modalState.modalType, modalState.extensionInfo);
  };

  const config = getModalConfig();
  if (!config) return null;

  const getConfirmButtonVariant = () => {
    switch (modalState.modalType) {
      case 'blocked':
        return 'outline';
      case 'untrusted':
        return 'destructive';
      case 'trusted':
      default:
        return 'default';
    }
  };

  const getTitleClassName = () => {
    switch (modalState.modalType) {
      case 'blocked':
        return 'text-red-600 dark:text-red-400';
      case 'untrusted':
        return 'text-yellow-600 dark:text-yellow-400';
      case 'trusted':
      default:
        return '';
    }
  };

  return (
    <Dialog open={modalState.isOpen} onOpenChange={(open) => !open && dismissModal()}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle className={getTitleClassName()}>{config.title}</DialogTitle>
          <DialogDescription className="whitespace-pre-wrap text-left">
            {config.message}
          </DialogDescription>
        </DialogHeader>

        <DialogFooter className="pt-4">
          {config.showSingleButton ? (
            <Button
              onClick={dismissModal}
              disabled={modalState.isPending}
              variant={getConfirmButtonVariant()}
            >
              {config.confirmLabel}
            </Button>
          ) : (
            <>
              <Button variant="outline" onClick={dismissModal} disabled={modalState.isPending}>
                {config.cancelLabel}
              </Button>
              <Button
                onClick={confirmInstall}
                disabled={modalState.isPending}
                variant={getConfirmButtonVariant()}
              >
                {modalState.isPending ? 'Installing...' : config.confirmLabel}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
