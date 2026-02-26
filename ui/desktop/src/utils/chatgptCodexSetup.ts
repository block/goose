import { configureProviderOauth } from '@/api';

export async function startChatGptCodexSetup(): Promise<{ success: boolean; message: string }> {
  try {
    await configureProviderOauth({
      path: { name: 'chatgpt_codex' },
      throwOnError: true,
    });
    return { success: true, message: 'ChatGPT Codex setup completed' };
  } catch (e) {
    return {
      success: false,
      message: `Failed to start ChatGPT Codex setup: ${e}`,
    };
  }
}
