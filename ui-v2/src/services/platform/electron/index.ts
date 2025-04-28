import { ElectronPlatformService } from './PlatformService';
import type { IPlatformService } from '../IPlatformService';

export const platformService = new ElectronPlatformService();
export type { IPlatformService };
