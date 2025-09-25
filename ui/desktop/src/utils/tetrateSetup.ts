import { startTetrateSetup as startTetrateSetupApi } from '../api';

export interface TetrateSetupStatus {
  isRunning: boolean;
  error: string | null;
}

export async function startTetrateSetup(): Promise<{ success: boolean; message: string }> {
  try {
    return (await startTetrateSetupApi({ throwOnError: true })).data;
  } catch (e) {
    return {
      success: false,
      message: `Failed to start Tetrate setup ['${e}]`,
    };
  }
}
