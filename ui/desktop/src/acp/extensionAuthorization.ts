import type { ExtensionAuthorizationRequiredNotification_unstable } from '@aaif/goose-sdk';
import { toastService } from '../toasts';

const pendingAuthorizationToasts = new Map<string, string | number>();

/**
 * The agent cannot reach the user's browser — it may be running in a container
 * or over a remote transport — so it asks us to open the authorization URL and
 * then blocks on its loopback callback until the user finishes or it times out.
 */
export async function openExtensionAuthorizationUrl(
  notification: ExtensionAuthorizationRequiredNotification_unstable
): Promise<void> {
  const { extensionName, authorizationUrl } = notification;
  dismissExtensionAuthorizationToast(extensionName);

  const toastId = toastService.loading({
    title: extensionName,
    msg: 'Finish signing in in your browser',
  });
  if (toastId !== undefined) {
    pendingAuthorizationToasts.set(extensionName, toastId);
  }

  await window.electron.openExternal(authorizationUrl);
}

export function dismissExtensionAuthorizationToast(extensionName: string): void {
  const toastId = pendingAuthorizationToasts.get(extensionName);
  if (toastId === undefined) {
    return;
  }
  toastService.dismiss(toastId);
  pendingAuthorizationToasts.delete(extensionName);
}
