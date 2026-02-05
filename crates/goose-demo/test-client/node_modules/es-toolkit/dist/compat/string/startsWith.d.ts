/**
 * Checks if a string contains another string at the beginning of the string.
 *
 * Checks if one string startsWith another string. Optional position parameter to start searching from a certain index.
 *
 * @param {string} str - The string that might contain the target string.
 * @param {string} target - The string to search for.
 * @param {number} position - An optional offset to start searching in the str string
 * @returns {boolean} - True if the str string starts with the target string.
 *
 * @example
 * const isPrefix = startsWith('fooBar', 'foo') // returns true
 * const isPrefix = startsWith('fooBar', 'bar') // returns false
 * const isPrefix = startsWith('fooBar', 'abc') // returns false
 * const isPrefix = startsWith('fooBar', 'Bar', 2) // returns true
 * const isPrefix = startsWith('fooBar', 'Bar', 5) // returns false
 */
declare function startsWith(str?: string, target?: string, position?: number): boolean;

export { startsWith };
