import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export const copyToClipboard = (text: string) => {
  if (window === undefined) return;
  window.navigator.clipboard.writeText(text);
};

export function getComponentName(name: string) {
  // convert kebab-case to title case
  return name.replace(/-/g, " ");
}

export function getRandomIndex(array: any[]) {
  return Math.floor(Math.random() * array.length);
}
