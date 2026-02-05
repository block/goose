/**
 * Repeats the given string n times.
 *
 * If n is less than 1, an empty string is returned, or if the string is an empty string,
 * the original string is returned unchanged.
 *
 * @param {string} str - The string to repeat.
 * @param {number} n - The number of times to repeat the string.
 * @returns {string} - The repeated string, or an empty string if n is less than 1.
 *
 * @example
 * repeat('abc', 0); // ''
 * repeat('abc', 2); // 'abcabc'
 */
declare function repeat(str?: string, n?: number): string;

export { repeat };
