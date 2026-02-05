/**
 * Pads string on the left and right sides if it's shorter than length. Padding characters are truncated if they can't be evenly divided by length.
 * If the length is less than or equal to the original string's length, or if the padding character is an empty string, the original string is returned unchanged.
 *
 * @param {string} str - The string to pad.
 * @param {number} [length] - The length of the resulting string once padded.
 * @param {string} [chars] - The character(s) to use for padding.
 * @returns {string} - The padded string, or the original string if padding is not required.
 *
 * @example
 * const result1 = pad('abc', 8);         // result will be '  abc   '
 * const result2 = pad('abc', 8, '_-');   // result will be '_-abc_-_'
 * const result3 = pad('abc', 3);         // result will be 'abc'
 * const result4 = pad('abc', 2);         // result will be 'abc'
 *
 */
declare function pad(str?: string, length?: number, chars?: string): string;

export { pad };
