import { ValueIteratee } from '../_internal/ValueIteratee.js';

/**
 * This method is like `sortedIndex` except that it accepts `iteratee`
 * which is invoked for `value` and each element of `array` to compute their
 * sort ranking. The iteratee is invoked with one argument: (value).
 *
 * @template T
 * @param {ArrayLike<T> | null | undefined} array - The sorted array to inspect.
 * @param {T} value - The value to evaluate.
 * @param {ValueIteratee<T>} [iteratee] - The iteratee invoked per element.
 * @returns {number} Returns the index at which `value` should be inserted into `array`.
 *
 * @example
 * const dict = { 'thirty': 30, 'forty': 40, 'fifty': 50 };
 * sortedIndexBy(['thirty', 'fifty'], 'forty', _.propertyOf(dict));
 * // => 1
 *
 * @example
 * sortedIndexBy([{ 'x': 4 }, { 'x': 5 }], { 'x': 4 }, 'x');
 * // => 0
 */
declare function sortedIndexBy<T>(array: ArrayLike<T> | null | undefined, value: T, iteratee?: ValueIteratee<T>): number;

export { sortedIndexBy };
