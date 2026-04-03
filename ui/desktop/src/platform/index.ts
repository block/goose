import type { PlatformAPI } from './types';
import { electronPlatform } from './electron';
import { webPlatform } from './web';

export type { PlatformAPI } from './types';
export type {
  NotificationData,
  MessageBoxOptions,
  MessageBoxResponse,
  SaveDialogOptions,
  SaveDialogResponse,
  FileResponse,
  OpenDialogReturnValue,
  CreateChatWindowOptions,
  UpdaterEvent,
  PlatformEventCallback,
} from './types';

const isElectron =
  typeof window !== 'undefined' &&
  typeof window.electron !== 'undefined';

export const platform: PlatformAPI = isElectron ? electronPlatform : webPlatform;
