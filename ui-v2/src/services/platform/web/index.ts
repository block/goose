import { WebPlatformService } from './PlatformService';
import type { IPlatformService } from '../IPlatformService';

export const platformService = new WebPlatformService();
export type { IPlatformService };
