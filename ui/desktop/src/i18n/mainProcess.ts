import { resolveLanguage, normalizeUiLanguageSetting, SupportedLanguage } from './language';
import type { Settings } from '../utils/settings';

type MainTranslationKey =
  | 'nativeMenu.newWindow'
  | 'nativeMenu.settings'
  | 'nativeMenu.find'
  | 'nativeMenu.findEllipsis'
  | 'nativeMenu.findNext'
  | 'nativeMenu.findPrevious'
  | 'nativeMenu.useSelectionForFind'
  | 'nativeMenu.newChat'
  | 'nativeMenu.newChatWindow'
  | 'nativeMenu.openDirectory'
  | 'nativeMenu.recentDirectories'
  | 'nativeMenu.focusGooseWindow'
  | 'nativeMenu.quickLauncher'
  | 'nativeMenu.window'
  | 'nativeMenu.alwaysOnTop'
  | 'nativeMenu.toggleNavigation'
  | 'nativeMenu.help'
  | 'nativeMenu.aboutGoose'
  | 'nativeMenu.version'
  | 'nativeNotification.openDirectoryFailedTitle'
  | 'nativeNotification.openDirectoryFailedBody'
  | 'nativeDialog.externalBackendUnreachableTitle'
  | 'nativeDialog.externalBackendUnreachableMessage'
  | 'nativeDialog.externalBackendUnreachableDetail'
  | 'nativeDialog.externalBackendDisableAndRetry'
  | 'nativeDialog.quit'
  | 'nativeDialog.gooseFailedToStartTitle'
  | 'nativeDialog.gooseFailedToStartMessage'
  | 'nativeDialog.ok'
  | 'nativeDialog.appStartupErrorTitle'
  | 'nativeDialog.appStartupErrorMessage';

type TranslationParams = Record<string, string | number>;

const mainResources: Record<SupportedLanguage, Record<MainTranslationKey, string>> = {
  en: {
    'nativeMenu.newWindow': 'New Window',
    'nativeMenu.settings': 'Settings',
    'nativeMenu.find': 'Find',
    'nativeMenu.findEllipsis': 'Find…',
    'nativeMenu.findNext': 'Find Next',
    'nativeMenu.findPrevious': 'Find Previous',
    'nativeMenu.useSelectionForFind': 'Use Selection for Find',
    'nativeMenu.newChat': 'New Chat',
    'nativeMenu.newChatWindow': 'New Chat Window',
    'nativeMenu.openDirectory': 'Open Directory...',
    'nativeMenu.recentDirectories': 'Recent Directories',
    'nativeMenu.focusGooseWindow': 'Focus Goose Window',
    'nativeMenu.quickLauncher': 'Quick Launcher',
    'nativeMenu.window': 'Window',
    'nativeMenu.alwaysOnTop': 'Always on Top',
    'nativeMenu.toggleNavigation': 'Toggle Navigation',
    'nativeMenu.help': 'Help',
    'nativeMenu.aboutGoose': 'About goose',
    'nativeMenu.version': 'Version {{version}}',
    'nativeNotification.openDirectoryFailedTitle': 'goose',
    'nativeNotification.openDirectoryFailedBody': 'Could not open directory: {{directoryName}}',
    'nativeDialog.externalBackendUnreachableTitle': 'External Backend Unreachable',
    'nativeDialog.externalBackendUnreachableMessage':
      'Could not connect to external backend at {{backendUrl}}',
    'nativeDialog.externalBackendUnreachableDetail':
      'The external goosed server may not be running.',
    'nativeDialog.externalBackendDisableAndRetry': 'Disable External Backend & Retry',
    'nativeDialog.quit': 'Quit',
    'nativeDialog.gooseFailedToStartTitle': 'goose Failed to Start',
    'nativeDialog.gooseFailedToStartMessage': 'The backend server failed to start.',
    'nativeDialog.ok': 'OK',
    'nativeDialog.appStartupErrorTitle': 'goose Error',
    'nativeDialog.appStartupErrorMessage': 'Failed to create main window: {{error}}',
  },
  'zh-Hans': {
    'nativeMenu.newWindow': '新建窗口',
    'nativeMenu.settings': '设置',
    'nativeMenu.find': '查找',
    'nativeMenu.findEllipsis': '查找…',
    'nativeMenu.findNext': '查找下一个',
    'nativeMenu.findPrevious': '查找上一个',
    'nativeMenu.useSelectionForFind': '使用所选内容查找',
    'nativeMenu.newChat': '新建聊天',
    'nativeMenu.newChatWindow': '新建聊天窗口',
    'nativeMenu.openDirectory': '打开目录...',
    'nativeMenu.recentDirectories': '最近目录',
    'nativeMenu.focusGooseWindow': '聚焦 goose 窗口',
    'nativeMenu.quickLauncher': '快速启动器',
    'nativeMenu.window': '窗口',
    'nativeMenu.alwaysOnTop': '窗口置顶',
    'nativeMenu.toggleNavigation': '切换导航',
    'nativeMenu.help': '帮助',
    'nativeMenu.aboutGoose': '关于 goose',
    'nativeMenu.version': '版本 {{version}}',
    'nativeNotification.openDirectoryFailedTitle': 'goose',
    'nativeNotification.openDirectoryFailedBody': '无法打开目录：{{directoryName}}',
    'nativeDialog.externalBackendUnreachableTitle': '外部后端不可达',
    'nativeDialog.externalBackendUnreachableMessage': '无法连接到外部后端：{{backendUrl}}',
    'nativeDialog.externalBackendUnreachableDetail': '外部 goosed 服务可能未运行。',
    'nativeDialog.externalBackendDisableAndRetry': '禁用外部后端并重试',
    'nativeDialog.quit': '退出',
    'nativeDialog.gooseFailedToStartTitle': 'goose 启动失败',
    'nativeDialog.gooseFailedToStartMessage': '后端服务启动失败。',
    'nativeDialog.ok': '确定',
    'nativeDialog.appStartupErrorTitle': 'goose 错误',
    'nativeDialog.appStartupErrorMessage': '创建主窗口失败：{{error}}',
  },
  'zh-Hant': {
    'nativeMenu.newWindow': '新增視窗',
    'nativeMenu.settings': '設定',
    'nativeMenu.find': '尋找',
    'nativeMenu.findEllipsis': '尋找…',
    'nativeMenu.findNext': '尋找下一個',
    'nativeMenu.findPrevious': '尋找上一個',
    'nativeMenu.useSelectionForFind': '使用選取內容尋找',
    'nativeMenu.newChat': '新增聊天',
    'nativeMenu.newChatWindow': '新增聊天視窗',
    'nativeMenu.openDirectory': '開啟目錄...',
    'nativeMenu.recentDirectories': '最近目錄',
    'nativeMenu.focusGooseWindow': '聚焦 goose 視窗',
    'nativeMenu.quickLauncher': '快速啟動器',
    'nativeMenu.window': '視窗',
    'nativeMenu.alwaysOnTop': '視窗置頂',
    'nativeMenu.toggleNavigation': '切換導覽',
    'nativeMenu.help': '說明',
    'nativeMenu.aboutGoose': '關於 goose',
    'nativeMenu.version': '版本 {{version}}',
    'nativeNotification.openDirectoryFailedTitle': 'goose',
    'nativeNotification.openDirectoryFailedBody': '無法開啟目錄：{{directoryName}}',
    'nativeDialog.externalBackendUnreachableTitle': '外部後端無法連線',
    'nativeDialog.externalBackendUnreachableMessage': '無法連線到外部後端：{{backendUrl}}',
    'nativeDialog.externalBackendUnreachableDetail': '外部 goosed 服務可能尚未執行。',
    'nativeDialog.externalBackendDisableAndRetry': '停用外部後端並重試',
    'nativeDialog.quit': '結束',
    'nativeDialog.gooseFailedToStartTitle': 'goose 啟動失敗',
    'nativeDialog.gooseFailedToStartMessage': '後端服務啟動失敗。',
    'nativeDialog.ok': '確定',
    'nativeDialog.appStartupErrorTitle': 'goose 錯誤',
    'nativeDialog.appStartupErrorMessage': '建立主視窗失敗：{{error}}',
  },
};

function formatTemplate(template: string, params?: TranslationParams): string {
  if (!params) {
    return template;
  }
  return template.replace(/\{\{(\w+)\}\}/g, (_, key: string) => String(params[key] ?? ''));
}

export function resolveMainLanguage(
  uiLanguage: Settings['uiLanguage'],
  systemLocale: string | undefined
): SupportedLanguage {
  const normalizedSetting = normalizeUiLanguageSetting(uiLanguage);
  return resolveLanguage(normalizedSetting, systemLocale);
}

export function translateMain(
  key: MainTranslationKey,
  uiLanguage: Settings['uiLanguage'],
  systemLocale: string | undefined,
  params?: TranslationParams
): string {
  const language = resolveMainLanguage(uiLanguage, systemLocale);
  const template = mainResources[language][key] ?? mainResources.en[key] ?? key;
  return formatTemplate(template, params);
}
