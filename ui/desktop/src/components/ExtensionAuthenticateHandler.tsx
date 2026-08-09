import { useCallback, useEffect } from 'react';
import { IpcRendererEvent } from 'electron';
import { authenticateExtensionFromDeepLink } from './settings/extensions/extensionAuthDeeplink';

/**
 * Handles avocado-work://extension-authenticate?configKey=… deeplinks from the OS.
 */
export function ExtensionAuthenticateHandler() {
  const handleAuthenticateExtension = useCallback(
    async (_event: IpcRendererEvent, url: unknown) => {
      if (typeof url !== 'string') {
        return;
      }
      try {
        await authenticateExtensionFromDeepLink(url);
      } catch (error) {
        console.error('Failed to authenticate extension from deeplink:', error);
      }
    },
    []
  );

  useEffect(() => {
    window.electron.on('authenticate-extension', handleAuthenticateExtension);
    return () => {
      window.electron.off('authenticate-extension', handleAuthenticateExtension);
    };
  }, [handleAuthenticateExtension]);

  return null;
}
