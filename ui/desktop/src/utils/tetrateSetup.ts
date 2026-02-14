export async function startTetrateSetup(): Promise<{
  success: boolean;
  message: string;
}> {
  try {
    return await window.electron.startTetrateAuth();
  } catch (e) {
    return {
      success: false,
      message: `Failed to start Tetrate setup: ${e}`,
    };
  }
}

export async function cancelTetrateSetup(): Promise<boolean> {
  try {
    return await window.electron.cancelTetrateAuth();
  } catch {
    return false;
  }
}
