/**
 * Pads the end of a string with a given character until it reaches the specified length.
 *
 * If the length is less than or equal to the original string's length, or if the padding character is an empty string,
 * the original string is returned unchanged.
 *
 * @param {string} str - The string to pad.
 * @param {number} [length] - The length of the resulting string once padded.
 * @param {string} [chars] - The character(s) to use for padding.
 * @returns {string} - The padded string, or the original string if padding is not required.
 *
 * @example
 * const result1 = padEnd('abc', 6);          // result will be 'abc   '
 * const result2 = padEnd('abc', 6, '_-');    // result will be 'abc_-_'
 * const result3 = padEnd('abc', 3);          // result will be 'abc'
 * const result4 = padEnd('abc', 2);          // result will be 'abc'
 */
declare function padEnd(str?: string, length?: number, chars?: string): string;

export { padEnd };
