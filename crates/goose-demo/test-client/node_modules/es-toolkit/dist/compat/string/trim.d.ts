/**
 * Removes leading and trailing whitespace or specified characters from a string.
 *
 * @param {string} str - The string from which leading and trailing characters will be trimmed.
 * @param {string | string[]} chars - The character(s) to remove from the end of the string. Defaults to `" "`.
 * @returns {string} - The resulting string after the specified leading and trailing characters have been removed.
 *
 * @example
 * trim("  hello  "); // "hello"
 * trim("--hello--", "-"); // "hello"
 * trim("##hello##", ["#", "o"]); // "hell"
 */
declare function trim(string?: string, chars?: string): string;
/**
 * Removes leading and trailing whitespace or specified characters from a string.
 *
 * @param {string} str - The string from which leading and trailing characters will be trimmed.
 * @param {string | string[]} chars - The character(s) to remove from the end of the string. Defaults to `" "`.
 * @returns {string} - The resulting string after the specified leading and trailing characters have been removed.
 *
 * @example
 * trim("  hello  "); // "hello"
 * trim("--hello--", "-"); // "hello"
 * trim("##hello##", ["#", "o"]); // "hell"
 */
declare function trim(string: string, index: string | number, guard: object): string;

export { trim };
