export interface TetrateSetupStatus {
  isRunning: boolean;
  error: string | null;
}

export async function startTetrateSetup(): Promise<{ success: boolean; message: string }> {
  const baseUrl = `${window.appConfig.get('GOOSE_API_HOST')}:${window.appConfig.get('GOOSE_PORT')}`;
  const response = await fetch(`${baseUrl}/handle_tetrate`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
  });

  if (!response.ok) {
    console.error('Failed to start Tetrate setup:', response.statusText);
    return {
      success: false,
      message: `Failed to start Tetrate setup ['${response.status}]`,
    };
  }

  const result = await response.json();
  return result;
}
