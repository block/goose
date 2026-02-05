/**
 * Checks if a string contains another string at the end of the string.
 *
 * Checks if one string endsWith another string. Optional position parameter to offset searching before a certain index.
 *
 * @param {string} str - The string that might contain the target string.
 * @param {string} target - The string to search for.
 * @param {number} position - An optional position from the start to search up to this index
 * @returns {boolean} - True if the str string ends with the target string.
 *
 * @example
 * const isPrefix = endsWith('fooBar', 'foo') // returns true
 * const isPrefix = endsWith('fooBar', 'bar') // returns false
 * const isPrefix = endsWith('fooBar', 'abc') // returns false
 * const isPrefix = endsWith('fooBar', 'foo', 3) // returns true
 * const isPrefix = endsWith('fooBar', 'abc', 5) // returns false
 */
declare function endsWith(str?: string, target?: string, position?: number): boolean;

export { endsWith };
