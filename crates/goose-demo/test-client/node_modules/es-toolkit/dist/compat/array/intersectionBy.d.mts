import { ValueIteratee } from '../_internal/ValueIteratee.mjs';

/**
 * Creates an array of unique values that are included in all given arrays, using an iteratee to compute equality.
 *
 * @template T, U
 * @param {ArrayLike<T> | null} array - The array to inspect.
 * @param {ArrayLike<U>} values - The values to compare.
 * @param {ValueIteratee<T | U>} iteratee - The iteratee invoked per element.
 * @returns {T[]} Returns the new array of intersecting values.
 *
 * @example
 * intersectionBy([2.1, 1.2], [2.3, 3.4], Math.floor);
 * // => [2.1]
 */
declare function intersectionBy<T, U>(array: ArrayLike<T> | null, values: ArrayLike<U>, iteratee: ValueIteratee<T | U>): T[];
/**
 * Creates an array of unique values that are included in all given arrays, using an iteratee to compute equality.
 *
 * @template T, U, V
 * @param {ArrayLike<T> | null} array - The array to inspect.
 * @param {ArrayLike<U>} values1 - The first values to compare.
 * @param {ArrayLike<V>} values2 - The second values to compare.
 * @param {ValueIteratee<T | U | V>} iteratee - The iteratee invoked per element.
 * @returns {T[]} Returns the new array of intersecting values.
 *
 * @example
 * intersectionBy([2.1, 1.2], [2.3, 3.4], [2.5], Math.floor);
 * // => [2.1]
 */
declare function intersectionBy<T, U, V>(array: ArrayLike<T> | null, values1: ArrayLike<U>, values2: ArrayLike<V>, iteratee: ValueIteratee<T | U | V>): T[];
/**
 * Creates an array of unique values that are included in all given arrays, using an iteratee to compute equality.
 *
 * @template T, U, V, W
 * @param {ArrayLike<T> | null | undefined} array - The array to inspect.
 * @param {ArrayLike<U>} values1 - The first values to compare.
 * @param {ArrayLike<V>} values2 - The second values to compare.
 * @param {...Array<ArrayLike<W> | ValueIteratee<T | U | V | W>>} values - The other arrays to compare, and the iteratee to use.
 * @returns {T[]} Returns the new array of intersecting values.
 *
 * @example
 * intersectionBy([2.1, 1.2], [2.3, 3.4], [2.5], [2.6, 1.7], Math.floor);
 * // => [2.1]
 */
declare function intersectionBy<T, U, V, W>(array: ArrayLike<T> | null | undefined, values1: ArrayLike<U>, values2: ArrayLike<V>, ...values: Array<ArrayLike<W> | ValueIteratee<T | U | V | W>>): T[];
/**
 * Creates an array of unique values that are included in all given arrays.
 *
 * @template T
 * @param {ArrayLike<T> | null} [array] - The array to inspect.
 * @param {...Array<ArrayLike<T>>} values - The values to compare.
 * @returns {T[]} Returns the new array of intersecting values.
 *
 * @example
 * intersectionBy([2, 1], [2, 3]);
 * // => [2]
 */
declare function intersectionBy<T>(array?: ArrayLike<T> | null, ...values: Array<ArrayLike<T>>): T[];
/**
 * Creates an array of unique values that are included in all given arrays, using an iteratee to compute equality.
 *
 * @template T
 * @param {...Array<ArrayLike<T> | ValueIteratee<T>>} values - The arrays to compare and the iteratee to use.
 * @returns {T[]} Returns the new array of intersecting values.
 *
 * @example
 * intersectionBy([2.1, 1.2], [2.3, 3.4], Math.floor);
 * // => [2.1]
 */
declare function intersectionBy<T>(...values: Array<ArrayLike<T> | ValueIteratee<T>>): T[];

export { intersectionBy };
