export interface OpenAISetupStatus {
  isRunning: boolean;
  error: string | null;
}

export async function startOpenAISetup(): Promise<{ success: boolean; message: string; requiresManualSetup?: boolean }> {
  // OpenAI doesn't have OAuth flow, so we redirect to manual setup
  return {
    success: true,
    message: "Redirecting to OpenAI configuration...",
    requiresManualSetup: true
  };
}
