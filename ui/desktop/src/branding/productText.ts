export const DEFAULT_APP_NAME = 'Goose';

const BRANDING_SKIP_PATTERNS = [/goose:\/\//i, /\.goosehints\b/i, /\bgoosed\b/i];

export function resolveProductName(value: unknown): string {
  return typeof value === 'string' && value.trim() ? value.trim() : DEFAULT_APP_NAME;
}

export function getConfiguredProductName(): string {
  if (typeof window === 'undefined' || !window.appConfig) {
    return DEFAULT_APP_NAME;
  }

  return resolveProductName(window.appConfig.get('GOOSE_APP_NAME'));
}

export function getOnboardingTitle(appName: string): string {
  return `Welcome to ${resolveProductName(appName)}`;
}

export function getOnboardingDescription(appName: string): string {
  return `Your local AI agent. Connect an AI model provider to get started with ${resolveProductName(appName)}.`;
}

export function getLauncherPlaceholder(appName: string): string {
  return `Ask ${resolveProductName(appName)} anything...`;
}

export function getTaskCompleteTitle(appName: string): string {
  return `${resolveProductName(appName)} finished the task.`;
}

export function getTaskCompleteBody(appName: string): string {
  return `Click here to bring ${resolveProductName(appName)} back into focus.`;
}

function brandMessageText(message: string, appName: string): string {
  if (!message || BRANDING_SKIP_PATTERNS.some((pattern) => pattern.test(message))) {
    return message;
  }

  return message.replace(/\bGoose\b/g, appName).replace(/\bgoose\b/g, appName);
}

export function brandMessageCatalog(
  messages: Record<string, string>,
  configuredAppName: string
): Record<string, string> {
  const appName = resolveProductName(configuredAppName);
  if (appName === DEFAULT_APP_NAME) {
    return messages;
  }

  return Object.fromEntries(
    Object.entries(messages).map(([key, message]) => [key, brandMessageText(message, appName)])
  );
}
