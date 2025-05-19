declare global {
  interface Window {
    electronAPI: {
      copyToClipboard: (text: string) => Promise<void>;
    };
  }
}

export class ElectronService {
  async copyToClipboard(text: string): Promise<void> {
    return window.electronAPI.copyToClipboard(text);
  }
}

export const electronService = new ElectronService();