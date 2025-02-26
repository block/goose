/**
 * Generate a random ID string
 * @returns A random string ID
 */
export function generateId(): string {
  return Math.random().toString(36).substring(2, 10);
}