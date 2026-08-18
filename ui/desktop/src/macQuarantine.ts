import { spawnSync } from 'node:child_process';

export type XattrRunner = (args: string[]) => { status: number | null };

function appBundleRoot(targetPath: string): string | null {
  const marker = '.app/';
  const index = targetPath.lastIndexOf(marker);
  if (index === -1) {
    return targetPath.endsWith('.app') ? targetPath : null;
  }
  return targetPath.slice(0, index + 4);
}

function defaultXattrRunner(args: string[]): { status: number | null } {
  return spawnSync('xattr', args, { encoding: 'utf8' });
}

/** Chrome-downloaded unsigned builds quarantine nested goose; spawn then hangs. */
export function clearMacQuarantine(
  targetPath: string,
  runXattr: XattrRunner = defaultXattrRunner
): boolean {
  if (process.platform !== 'darwin' || !targetPath) {
    return false;
  }

  const clearedBinary = runXattr(['-cr', targetPath]).status === 0;
  const bundle = appBundleRoot(targetPath);
  const clearedBundle =
    bundle && bundle !== targetPath ? runXattr(['-cr', bundle]).status === 0 : false;
  return clearedBinary || clearedBundle;
}
