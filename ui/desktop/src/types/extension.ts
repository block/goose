export type ModalType = 'blocked' | 'untrusted' | 'trusted';

export interface ExtensionInfo {
  name: string;
  command?: string;
  remoteUrl?: string;
  link: string;
}

export interface ExtensionModalState {
  isOpen: boolean;
  modalType: ModalType;
  extensionInfo: ExtensionInfo | null;
  isPending: boolean;
  error: string | null;
}

export interface ExtensionModalConfig {
  title: string;
  message: string;
  confirmLabel: string;
  cancelLabel: string;
  showSingleButton: boolean;
  isBlocked: boolean;
}

export interface ExtensionInstallResult {
  success: boolean;
  error?: string;
}
