/**
 * Removes trailing whitespace or specified characters from a string.
 *
 * @param {string} string - The string to trim.
 * @param {string} chars - The characters to trim from the end of the string.
 * @returns {string} Returns the trimmed string.
 *
 * @example
 * trimEnd('  abc  ');
 * // => '  abc'
 *
 * trimEnd('-_-abc-_-', '_-');
 * // => '-_-abc'
 */
declare function trimEnd(string?: string, chars?: string): string;
/**
 * Removes trailing whitespace or specified characters from a string.
 *
 * @param {string} string - The string to trim.
 * @param {string | number} index - The index parameter (used with guard).
 * @param {object} guard - Enables use as an iteratee for methods like `map`.
 * @returns {string} Returns the trimmed string.
 *
 * @example
 * trimEnd('  abc  ', 0, {});
 * // => '  abc'
 */
declare function trimEnd(string: string, index: string | number, guard: object): string;

export { trimEnd };
