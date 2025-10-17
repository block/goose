export interface AnthropicSetupStatus {
  isRunning: boolean;
  error: string | null;
}

export async function startAnthropicSetup(): Promise<{ success: boolean; message: string; requiresManualSetup?: boolean }> {
  // Anthropic doesn't have OAuth flow, so we redirect to manual setup
  return {
    success: true,
    message: "Redirecting to Anthropic configuration...",
    requiresManualSetup: true
  };
}
