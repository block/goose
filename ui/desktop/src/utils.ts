import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';
import { client } from './api/client.gen';

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
  return;
}

// This needs to be called before any API calls are made, but since we're using the client
// in multiple useEffect locations, we can't be sure who goes first.
let clientInitialized = false;

export async function ensureClientInitialized() {
  if (clientInitialized) return;
  client.setConfig({
    baseUrl: window.appConfig.get('GOOSE_API_HOST') + ':' + window.appConfig.get('GOOSE_PORT'),
    headers: {
      'Content-Type': 'application/json',
      'X-Secret-Key': await window.electron.getSecretKey(),
    },
  });
  clientInitialized = true;
}
