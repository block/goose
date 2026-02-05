/**
 * Removes leading whitespace or specified characters from a string.
 *
 * @param {string} string - The string to trim.
 * @param {string} chars - The characters to trim from the start of the string.
 * @returns {string} Returns the trimmed string.
 *
 * @example
 * trimStart('  abc  ');
 * // => 'abc  '
 *
 * trimStart('-_-abc-_-', '_-');
 * // => 'abc-_-'
 */
declare function trimStart(string?: string, chars?: string): string;
/**
 * Removes leading whitespace or specified characters from a string.
 *
 * @param {string} string - The string to trim.
 * @param {string | number} index - The index parameter (used with guard).
 * @param {object} guard - Enables use as an iteratee for methods like `map`.
 * @returns {string} Returns the trimmed string.
 *
 * @example
 * trimStart('  abc  ', 0, {});
 * // => 'abc  '
 */
declare function trimStart(string: string, index: string | number, guard: object): string;

export { trimStart };
