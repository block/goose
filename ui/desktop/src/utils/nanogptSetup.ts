import { startNanogptSetup as startNanogptSetupApi } from '../api';

export async function startNanogptSetup(): Promise<{ success: boolean; message: string }> {
  try {
    return (await startNanogptSetupApi({ throwOnError: true })).data;
  } catch (e) {
    return {
      success: false,
      message: `Failed to start NanoGPT setup ['${e}]`,
    };
  }
}
