export function homeDir(): string {
  // The renderer process does not have access to Node's `os` module.
  // Prefer Electron-provided info if available.
  //
  // - appConfig['HOME'] is a common pattern in this app (main process can inject env-like settings)
  // - Fall back to the configured GOOSE_WORKING_DIR if set
  // - Fall back to ~ as a final placeholder (used elsewhere for display logic)
  const fromConfig = window.appConfig?.get('HOME');
  if (typeof fromConfig === 'string' && fromConfig.length > 0) {
    return fromConfig;
  }

  const workingDir = window.appConfig?.get('GOOSE_WORKING_DIR');
  if (typeof workingDir === 'string' && workingDir.length > 0) {
    return workingDir;
  }

  return '~';
}
