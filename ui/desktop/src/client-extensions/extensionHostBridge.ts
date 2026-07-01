import { toastService } from '../toasts';
import { handleHostCapabilityMessage, notifyHostCapabilities } from './hostCapabilities';
import { isHostCapabilityInvokeMessage } from './hostCapabilities/types';
import type { ExtensionToHostMessage, HostCapability, HostToExtensionMessage } from './types';

export function isExtensionToHostMessage(value: unknown): value is ExtensionToHostMessage {
  if (typeof value !== 'object' || value === null || !('type' in value)) {
    return false;
  }
  const type = (value as { type: unknown }).type;
  if (typeof type !== 'string') {
    return false;
  }
  return (
    type === 'grc/ui/showMessage' ||
    type === 'grc/chat/setInput' ||
    type === 'grc/resize' ||
    isHostCapabilityInvokeMessage(value)
  );
}

export function notifyExtensionActivate(
  iframe: HTMLIFrameElement | null,
  message: HostToExtensionMessage,
  hostCapabilities: HostCapability[] | undefined
): void {
  if (!iframe?.contentWindow) {
    return;
  }

  iframe.contentWindow.postMessage(message, '*');
  notifyHostCapabilities(hostCapabilities, (payload) => {
    iframe.contentWindow?.postMessage(payload, '*');
  });
}

export async function routeExtensionToHostMessage(
  extensionId: string,
  hostCapabilities: HostCapability[] | undefined,
  message: ExtensionToHostMessage,
  postToExtension: (payload: unknown) => void,
  toastTitle: string
): Promise<boolean> {
  const handled = await handleHostCapabilityMessage(
    extensionId,
    hostCapabilities,
    message,
    postToExtension
  );
  if (handled) {
    return true;
  }

  switch (message.type) {
    case 'grc/ui/showMessage':
      toastService.success({
        title: toastTitle,
        msg: message.text,
      });
      return true;
    default:
      return false;
  }
}
