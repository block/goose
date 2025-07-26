export interface OpenRouterSetupStatus {
  isRunning: boolean;
  error: string | null;
}

export async function startOpenRouterSetup(): Promise<{ success: boolean; message: string }> {
  try {
    const baseUrl = `${window.appConfig.get('GOOSE_API_HOST')}:${window.appConfig.get('GOOSE_PORT')}`;
    const response = await fetch(`${baseUrl}/setup/openrouter/start`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
    });

    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }

    const result = await response.json();
    return result;
  } catch (error) {
    console.error('Failed to start OpenRouter setup:', error);
    return {
      success: false,
      message: error instanceof Error ? error.message : 'Failed to start OpenRouter setup',
    };
  }
}
