import { startOpenrouterSetup } from '../api';

export interface OpenRouterSetupStatus {
  isRunning: boolean;
  error: string | null;
}

export async function startOpenRouterSetup(): Promise<{ success: boolean; message: string }> {
  try {
    return (await startOpenrouterSetup({ throwOnError: true })).data;
  } catch (e) {
    return {
      success: false,
      message: `Failed to start Openrouter setup ['${e}]`,
    };
  }
}
