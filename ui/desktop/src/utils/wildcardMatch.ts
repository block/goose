/**
 * Utility for wildcard pattern matching using * as wildcard character
 */

/**
 * Converts a wildcard pattern to a regular expression for substring matching
 * @param pattern - The wildcard pattern with * as wildcard character
 * @returns A RegExp object that matches the pattern as a substring
 */
export function wildcardToRegExp(pattern: string): RegExp {
  // Escape special characters except for *
  const escaped = pattern.replace(/[.+?^${}()|[\]\\]/g, '\\$&');
  
  // Replace * with .*
  const regexPattern = escaped.replace(/\*/g, '.*');
  
  // Create the regex without anchors to match any part of the string
  return new RegExp(regexPattern);
}

/**
 * Tests if a string contains a substring that matches a wildcard pattern
 * @param str - The string to test
 * @param pattern - The wildcard pattern with * as wildcard character
 * @param caseSensitive - Whether the match should be case sensitive
 * @returns True if the string contains a substring matching the pattern, false otherwise
 */
export function wildcardMatch(str: string, pattern: string, caseSensitive: boolean = false): boolean {
  if (!pattern.includes('*')) {
    // If there's no wildcard, do a simple includes check
    return caseSensitive ? str.includes(pattern) : str.toLowerCase().includes(pattern.toLowerCase());
  }
  
  // Convert the wildcard pattern to a RegExp
  const regex = wildcardToRegExp(pattern);
  
  // Add case insensitivity flag if needed
  const flags = caseSensitive ? '' : 'i';
  const finalRegex = new RegExp(regex, flags);
  
  // Test the string against the RegExp
  return finalRegex.test(str);
}