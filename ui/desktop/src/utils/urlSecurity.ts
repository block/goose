// URL protocol constants and security utilities

// Protocols for web content only (HTTP requests, browser URLs, server connections)
export const WEB_PROTOCOLS = ['http:', 'https:'];

// Protocols that should never be opened (security risk)
export const BLOCKED_PROTOCOLS = [
  'file:',
  'javascript:',
  'data:',
  'vbscript:',
  'blob:',
  'about:',
  'chrome:',
  'chrome-extension:',
];

// Protocols that are safe to open without confirmation
export const SAFE_PROTOCOLS = [
  // Web
  'http:',
  'https:',
  // Communication
  'mailto:',
  'tel:',
  'sms:',
  'facetime:',
  'facetime-audio:',
  // Messaging apps
  'slack:',
  'discord:',
  'tg:', // Telegram
  'telegram:',
  'whatsapp:',
  'skype:',
  'msteams:', // Microsoft Teams
  // Development tools
  'vscode:',
  'vscode-insiders:',
  'vscodium:',
  'jetbrains:',
  'sublime:',
  'atom:',
  'github-mac:',
  'github-windows:',
  'sourcetree:',
  'cursor:',
  // Media
  'spotify:',
  'music:', // Apple Music
  'itmss:', // iTunes
  'vlc:',
  // Video conferencing
  'zoommtg:',
  'zoomus:',
  'webex:',
  'meet:', // Google Meet
  // Productivity
  'notion:',
  'obsidian:',
  'bear:',
  'things:',
  'omnifocus:',
  'todoist:',
  'evernote:',
  'onenote:',
  // Cloud storage
  'dropbox:',
  'googledrive:',
  'onedrive:',
  // Browsers
  'googlechrome:',
  'firefox:',
  'safari:',
  // Goose
  'goose:',
];

/**
 * Check if a URL uses a protocol that is safe to open without user confirmation.
 * Dangerous protocols are blocked centrally in main.ts open-external handler.
 */
export const isProtocolSafe = (url: string): boolean => {
  try {
    const parsed = new URL(url);
    return SAFE_PROTOCOLS.includes(parsed.protocol);
  } catch {
    return false;
  }
};

/**
 * Extract the protocol from a URL string.
 */
export const getProtocol = (url: string): string | null => {
  try {
    return new URL(url).protocol;
  } catch {
    return null;
  }
};
