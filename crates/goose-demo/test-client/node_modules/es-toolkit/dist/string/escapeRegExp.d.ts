/**
 * Escapes the RegExp special characters "^", "$", "\\", ".", "*", "+", "?", "(", ")", "[", "]", "{", "}", and "|" in `str`.
 *
 * @param {string} str The string to escape.
 * @returns {string} Returns the escaped string.
 *
 * @example
 * import { escapeRegExp } from 'es-toolkit/string';
 *
 * escapeRegExp('[es-toolkit](https://es-toolkit.dev/)'); // returns '\[es-toolkit\]\(https://es-toolkit\.dev/\)'
 */
declare function escapeRegExp(str: string): string;

export { escapeRegExp };
