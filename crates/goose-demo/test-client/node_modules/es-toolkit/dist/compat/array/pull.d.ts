/**
 * Removes all provided values from array using SameValueZero for equality comparisons.
 *
 * **Note:** Unlike `_.without`, this method mutates `array`.
 *
 * @template T
 * @param {T[]} array - The array to modify.
 * @param {...T[]} values - The values to remove.
 * @returns {T[]} Returns `array`.
 *
 * @example
 * var array = [1, 2, 3, 1, 2, 3];
 *
 * pull(array, 2, 3);
 * console.log(array);
 * // => [1, 1]
 */
declare function pull<T>(array: T[], ...values: T[]): T[];
/**
 * Removes all provided values from array using SameValueZero for equality comparisons.
 *
 * **Note:** Unlike `_.without`, this method mutates `array`.
 *
 * @template L
 * @param {L} array - The array to modify.
 * @param {...L[0][]} values - The values to remove.
 * @returns {L} Returns `array`.
 *
 * @example
 * var array = [1, 2, 3, 1, 2, 3];
 *
 * pull(array, 2, 3);
 * console.log(array);
 * // => [1, 1]
 */
declare function pull<L extends ArrayLike<any>>(array: L extends readonly any[] ? never : L, ...values: Array<L[0]>): L;

export { pull };
