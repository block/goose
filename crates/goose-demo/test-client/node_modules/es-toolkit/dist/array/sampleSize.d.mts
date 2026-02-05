/**
 * Returns a sample element array of a specified `size`.
 *
 * This function takes an array and a number, and returns an array containing the sampled elements using Floyd's algorithm.
 *
 * {@link https://www.nowherenearithaca.com/2013/05/robert-floyds-tiny-and-beautiful.html Floyd's algorithm}
 *
 * @template T - The type of elements in the array.
 * @param {T[]} array - The array to sample from.
 * @param {number} size - The size of sample.
 * @returns {T[]} A new array with sample size applied.
 * @throws {Error} Throws an error if `size` is greater than the length of `array`.
 *
 * @example
 * const result = sampleSize([1, 2, 3], 2)
 * // result will be an array containing two of the elements from the array.
 * // [1, 2] or [1, 3] or [2, 3]
 */
declare function sampleSize<T>(array: readonly T[], size: number): T[];

export { sampleSize };
