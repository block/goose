import type { PlatformAPI } from './types';

/**
 * Electron implementation — delegates everything to window.electron
 * which is set up by preload.ts via contextBridge.
 */
export const electronPlatform: PlatformAPI = {
  isWeb: false,

  get platform() {
    return window.electron.platform;
  },
  get arch() {
    return window.electron.arch;
  },

  reactReady: () => window.electron.reactReady(),
  getConfig: () => window.electron.getConfig(),
  reloadApp: () => window.electron.reloadApp(),

  getSecretKey: () => window.electron.getSecretKey(),
  getGoosedHostPort: () => window.electron.getGoosedHostPort(),

  hideWindow: () => window.electron.hideWindow(),
  closeWindow: () => window.electron.closeWindow(),
  createChatWindow: (options) => window.electron.createChatWindow(options),

  directoryChooser: () => window.electron.directoryChooser(),
  showMessageBox: (options) => window.electron.showMessageBox(options),
  showSaveDialog: (options) => window.electron.showSaveDialog(options),
  selectFileOrDirectory: (defaultPath) => window.electron.selectFileOrDirectory(defaultPath),

  readFile: (filePath) => window.electron.readFile(filePath),
  writeFile: (filePath, content) => window.electron.writeFile(filePath, content),
  ensureDirectory: (dirPath) => window.electron.ensureDirectory(dirPath),
  listFiles: (dirPath, extension) => window.electron.listFiles(dirPath, extension),
  getBinaryPath: (name) => window.electron.getBinaryPath(name),
  getPathForFile: (file) => window.electron.getPathForFile(file),
  getAllowedExtensions: () => window.electron.getAllowedExtensions(),
  openDirectoryInExplorer: (dir) => window.electron.openDirectoryInExplorer(dir),

  showNotification: (data) => window.electron.showNotification(data),
  logInfo: (txt) => window.electron.logInfo(txt),

  openExternal: (url) => window.electron.openExternal(url),
  openInChrome: (url) => window.electron.openInChrome(url),
  fetchMetadata: (url) => window.electron.fetchMetadata(url),

  getSetting: (key) => window.electron.getSetting(key),
  setSetting: (key, value) => window.electron.setSetting(key, value),

  setMenuBarIcon: (show) => window.electron.setMenuBarIcon(show),
  getMenuBarIconState: () => window.electron.getMenuBarIconState(),
  setDockIcon: (show) => window.electron.setDockIcon(show),
  getDockIconState: () => window.electron.getDockIconState(),
  setWakelock: (enable) => window.electron.setWakelock(enable),
  getWakelockState: () => window.electron.getWakelockState(),
  setSpellcheck: (enable) => window.electron.setSpellcheck(enable),
  getSpellcheckState: () => window.electron.getSpellcheckState(),
  openNotificationsSettings: () => window.electron.openNotificationsSettings(),

  on: (channel, callback) => window.electron.on(channel, callback as never),
  off: (channel, callback) => window.electron.off(channel, callback as never),
  emit: (channel, ...args) => window.electron.emit(channel, ...args),
  broadcastThemeChange: (data) => window.electron.broadcastThemeChange(data),
  onMouseBackButtonClicked: (cb) => window.electron.onMouseBackButtonClicked(cb),
  offMouseBackButtonClicked: (cb) => window.electron.offMouseBackButtonClicked(cb),

  getVersion: () => window.electron.getVersion(),
  checkForUpdates: () => window.electron.checkForUpdates(),
  downloadUpdate: () => window.electron.downloadUpdate(),
  installUpdate: () => window.electron.installUpdate(),
  restartApp: () => window.electron.restartApp(),
  onUpdaterEvent: (cb) => window.electron.onUpdaterEvent(cb),
  getUpdateState: () => window.electron.getUpdateState(),
  isUsingGitHubFallback: () => window.electron.isUsingGitHubFallback(),

  checkForOllama: () => window.electron.checkForOllama(),
  checkMesh: () => window.electron.checkMesh(),
  startMesh: (args) => window.electron.startMesh(args),
  stopMesh: () => window.electron.stopMesh(),

  hasAcceptedRecipeBefore: (recipe) => window.electron.hasAcceptedRecipeBefore(recipe),
  recordRecipeHash: (recipe) => window.electron.recordRecipeHash(recipe),

  launchApp: (app) => window.electron.launchApp(app),
  refreshApp: (app) => window.electron.refreshApp(app),
  closeApp: (name) => window.electron.closeApp(name),

  addRecentDir: (dir) => window.electron.addRecentDir(dir),
};
