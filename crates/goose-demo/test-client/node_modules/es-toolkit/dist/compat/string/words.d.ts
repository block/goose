/**
 * Splits `string` into an array of its words.
 *
 * @param {string | object} str - The string or object that is to be split into words.
 * @param {RegExp | string} [pattern] - The pattern to match words.
 * @returns {string[]} - Returns the words of `string`.
 *
 * @example
 * const wordsArray1 = words('fred, barney, & pebbles');
 * // => ['fred', 'barney', 'pebbles']
 *
 */
declare function words(string?: string, pattern?: string | RegExp): string[];
/**
 * Splits `string` into an array of its words.
 *
 * @param {string | object} str - The string or object that is to be split into words.
 * @param {RegExp | string} [pattern] - The pattern to match words.
 * @returns {string[]} - Returns the words of `string`.
 *
 * @example
 * const wordsArray1 = words('fred, barney, & pebbles');
 * // => ['fred', 'barney', 'pebbles']
 */
declare function words(string: string, index: string | number, guard: object): string[];

export { words };
