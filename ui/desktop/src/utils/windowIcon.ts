export function getWindowIconFilename(platform: NodeJS.Platform): string {
  if (platform === 'darwin') {
    return 'icon.icns';
  }
  if (platform === 'win32') {
    return 'icon.ico';
  }
  return 'icon.png';
}
