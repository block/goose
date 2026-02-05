/**
 * Splits `string` into an array of its words, treating spaces and punctuation marks as separators.
 *
 * @param {string} str The string to inspect.
 * @param {RegExp | string} [pattern] The pattern to match words.
 * @returns {string[]} Returns the words of `string`.
 *
 * @example
 * words('fred, barney, & pebbles');
 * // => ['fred', 'barney', 'pebbles']
 *
 * words('camelCaseHTTPRequest🚀');
 * // => ['camel', 'Case', 'HTTP', 'Request', '🚀']
 *
 * words('Lunedì 18 Set')
 * // => ['Lunedì', '18', 'Set']
 */
declare function words(str: string): string[];

export { words };
