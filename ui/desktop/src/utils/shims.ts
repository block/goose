export const SUPPORTED_EXTENSION_RUNNERS = ['jbang', 'npx', 'uvx'];
export const SHIM_COMMANDS = ['goosed', ...SUPPORTED_EXTENSION_RUNNERS];

/**
 * Replaces the command with the corresponding binary path for the shim if one is supported
 * @param cmd The command to potentially replace with a shim
 * @returns The path to the binary if it's a known shim, otherwise returns the original command
 */
export async function replaceWithShim(cmd: string): Promise<string> {
  if (SHIM_COMMANDS.indexOf(cmd) === -1) {
    return cmd;
  }
  const binaryPath = window.electron.getBinaryPath(cmd);
  console.log('--------> Replacing command with shim ------>', cmd, binaryPath);
  return binaryPath;
}
