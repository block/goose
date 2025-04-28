import { IPlatformService } from '../IPlatformService';

export class WebPlatformService implements IPlatformService {
  async copyToClipboard(text: string): Promise<void> {
    await navigator.clipboard.writeText(text);
  }
}
