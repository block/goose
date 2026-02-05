/**
 * Creates an array of numbers from `start` to `end` with optional `step`.
 * @param {number} start - The starting number of the range (inclusive).
 * @param {number} [end] - The end number of the range (exclusive).
 * @param {number} [step] - The step value for the range.
 * @returns {number[]} An array of numbers from `start` to `end` with the specified `step`.
 * @example
 * // Returns [0, 1, 2, 3]
 * rangeRight(4);
 * @example
 * // Returns [0, 2, 4, 6]
 * rangeRight(0, 8, 2);
 * @example
 * // Returns [5, 4, 3, 2, 1]
 * rangeRight(1, 6);
 */
declare function rangeRight(start: number, end?: number, step?: number): number[];
/**
 * Creates an array of numbers from 0 to `end` with step 1.
 * Used when called as an iteratee for methods like `_.map`.
 * @param {number} end - The end number of the range (exclusive).
 * @param {string | number} index - The index parameter (used for iteratee calls).
 * @param {object} guard - The guard parameter (used for iteratee calls).
 * @returns {number[]} An array of numbers from 0 to `end` with step 1.
 * @example
 * // Returns [0, 1, 2, 3]
 * rangeRight(4, 'index', {});
 */
declare function rangeRight(end: number, index: string | number, guard: object): number[];

export { rangeRight };
