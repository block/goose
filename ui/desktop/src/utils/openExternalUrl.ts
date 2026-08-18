import { dialog, shell, type BrowserWindow, type MessageBoxOptions } from 'electron';
import { BLOCKED_PROTOCOLS, SAFE_PROTOCOLS, type OpenExternalUrlResult } from './urlSecurity';

export const openExternalUrl = async (
  url: string,
  parentWindow?: BrowserWindow
): Promise<OpenExternalUrlResult> => {
  let protocol: string;
  try {
    protocol = new URL(url).protocol;
  } catch {
    return 'blocked';
  }

  if (BLOCKED_PROTOCOLS.includes(protocol)) return 'blocked';

  if (!SAFE_PROTOCOLS.includes(protocol)) {
    const options: MessageBoxOptions = {
      type: 'warning',
      buttons: ['Cancel', 'Open'],
      defaultId: 0,
      cancelId: 0,
      title: 'Open external link?',
      message: `Open ${protocol} link?`,
      detail: url,
    };
    const result = parentWindow
      ? await dialog.showMessageBox(parentWindow, options)
      : await dialog.showMessageBox(options);
    if (result.response !== 1) return 'cancelled';
  }

  await shell.openExternal(url);
  return 'opened';
};
