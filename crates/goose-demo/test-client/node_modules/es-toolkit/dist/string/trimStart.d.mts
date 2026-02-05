/**
 * Removes leading whitespace or specified characters from a string.
 *
 * If `chars` is a string, it should be a single character. To trim a string with multiple characters,
 * provide an array instead.
 *
 * @param {string} str - The string from which leading characters will be trimmed.
 * @param {string | string[]} chars - The character(s) to remove from the start of the string.
 * @returns {string} - The resulting string after the specified leading character has been removed.
 *
 * @example
 * const trimmedStr1 = trimStart('---hello', '-') // returns 'hello'
 * const trimmedStr2 = trimStart('000123', '0') // returns '123'
 * const trimmedStr3 = trimStart('abcabcabc', 'a') // returns 'bcabcabc'
 * const trimmedStr4 = trimStart('xxxtrimmed', 'x') // returns 'trimmed'
 */
declare function trimStart(str: string, chars?: string | string[]): string;

export { trimStart };
