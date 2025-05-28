import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}

export const copyToClipboard = (text: string): void => {
  if (window === undefined) return;
  window.navigator.clipboard.writeText(text);
};

export function getComponentName(name: string): string {
  // convert kebab-case to title case
  return name.replace(/-/g, ' ');
}

export function getRandomIndex<T>(array: T[]): number {
  return Math.floor(Math.random() * array.length);
}
