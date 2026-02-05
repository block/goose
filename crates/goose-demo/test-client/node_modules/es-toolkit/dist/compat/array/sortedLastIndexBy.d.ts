import { ValueIteratee } from '../_internal/ValueIteratee.js';

/**
 * This method is like `sortedLastIndex` except that it accepts `iteratee`
 * which is invoked for `value` and each element of `array` to compute their
 * sort ranking. The iteratee is invoked with one argument: (value).
 *
 * @template T
 * @param {ArrayLike<T> | null | undefined} array - The sorted array to inspect.
 * @param {T} value - The value to evaluate.
 * @param {ValueIteratee<T>} iteratee - The iteratee invoked per element.
 * @returns {number} Returns the index at which `value` should be inserted into `array`.
 *
 * @example
 * sortedLastIndexBy([{ 'x': 4 }, { 'x': 5 }], { 'x': 4 }, 'x');
 * // => 1
 */
declare function sortedLastIndexBy<T>(array: ArrayLike<T> | null | undefined, value: T, iteratee: ValueIteratee<T>): number;

export { sortedLastIndexBy };
