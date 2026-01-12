export async function startTetrateSetup(): Promise<{
  success: boolean;
  message: string;
}> {
  try {
    return await window.electron.startTetrateAuth();
  } catch (e) {
    return {
      success: false,
      message: `Failed to start Tetrate setup ['${e}]`,
    };
  }
}
