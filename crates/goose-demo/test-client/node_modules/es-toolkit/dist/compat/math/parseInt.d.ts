/**
 * Converts `string` to an integer of the specified radix. If `radix` is undefined or 0, a `radix` of 10 is used unless `string` is a hexadecimal, in which case a `radix` of 16 is used.
 *
 * @param {string} string The string to convert to an integer.
 * @param {number} radix The radix to use when converting the string to an integer. Defaults to `0`.
 * @param {unknown} guard Enables use as an iteratee for methods like `Array#map`.
 * @returns {number} Returns the converted integer.
 *
 * @example
 * parseInt('08'); // => 8
 * parseInt('0x20'); // => 32
 *
 * parseInt('08', 10); // => 8
 * parseInt('0x20', 16); // => 32
 *
 * ['6', '08', '10'].map(parseInt); // => [6, 8, 10]
 */
declare function parseInt(string: string, radix?: number): number;

export { parseInt };
