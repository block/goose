/**
 * Creates an array of numbers progressing from `start` up to, but not including, `end`.
 *
 * @param {number} start - The starting number of the range (inclusive)
 * @param {number} end - The end number of the range (exclusive)
 * @param {number} step - The value to increment or decrement by
 * @returns {number[]} An array of numbers from start to end
 * @example
 * range(4)
 * // => [0, 1, 2, 3]
 *
 * range(1, 5)
 * // => [1, 2, 3, 4]
 *
 * range(0, 20, 5)
 * // => [0, 5, 10, 15]
 */
declare function range(start: number, end?: number, step?: number): number[];
/**
 * Creates an array of numbers progressing from 0 up to, but not including, `end`.
 * Used internally when range is called as an iteratee.
 *
 * @param {number} end - The end of the range (exclusive)
 * @param {string|number} index - The index argument passed to the iteratee
 * @param {object} guard - The guard object passed to the iteratee
 * @returns {number[]} An array of numbers from 0 to end
 * @example
 * [1, 2, 3].map(range)
 * // => [[0], [0, 1], [0, 1, 2]]
 */
declare function range(end: number, index: string | number, guard: object): number[];

export { range };
