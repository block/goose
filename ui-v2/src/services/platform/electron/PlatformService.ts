import { IPlatformService } from '../IPlatformService';

declare global {
  interface Window {
    electronAPI: {
      copyToClipboard: (text: string) => Promise<void>;
    };
  }
}

export class ElectronPlatformService implements IPlatformService {
  async copyToClipboard(text: string): Promise<void> {
    return window.electronAPI.copyToClipboard(text);
  }
}
