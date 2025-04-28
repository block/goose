export interface IPlatformService {
  // Clipboard operations
  copyToClipboard(text: string): Promise<void>;
}
