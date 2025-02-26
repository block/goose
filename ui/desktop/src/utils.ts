import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function snakeToTitleCase(snake: string): string {
  return snake
    .split('_')
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
    .join(' ');
}

export function patchConsoleLogging() {
  // Intercept console methods
  const originalConsole = {
    log: console.log,
    error: console.error,
    warn: console.warn,
    info: console.info,
  };

  console.log = (...args: any[]) => {
    window.electron.logInfo(`[LOG] ${args.join(' ')}`);
    originalConsole.log(...args);
  };

  console.error = (...args: any[]) => {
    window.electron.logInfo(`[ERROR] ${args.join(' ')}`);
    originalConsole.error(...args);
  };

  console.warn = (...args: any[]) => {
    window.electron.logInfo(`[WARN] ${args.join(' ')}`);
    originalConsole.warn(...args);
  };

  console.info = (...args: any[]) => {
    window.electron.logInfo(`[INFO] ${args.join(' ')}`);
    originalConsole.info(...args);
  };
}
