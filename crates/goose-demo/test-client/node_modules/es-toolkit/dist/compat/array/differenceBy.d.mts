import { ValueIteratee } from '../_internal/ValueIteratee.mjs';

/**
 * Creates an array of array values not included in the other given arrays using an iteratee function.
 *
 * @template T1, T2
 * @param {ArrayLike<T1> | null | undefined} array The array to inspect
 * @param {ArrayLike<T2>} values The values to exclude
 * @param {ValueIteratee<T1 | T2>} iteratee The iteratee invoked per element
 * @returns {T1[]} Returns the new array of filtered values
 * @example
 * differenceBy([2.1, 1.2], [2.3, 3.4], Math.floor)
 * // => [1.2]
 */
declare function differenceBy<T1, T2>(array: ArrayLike<T1> | null | undefined, values: ArrayLike<T2>, iteratee: ValueIteratee<T1 | T2>): T1[];
/**
 * Creates an array of array values not included in the other given arrays using an iteratee function.
 *
 * @template T1, T2, T3
 * @param {ArrayLike<T1> | null | undefined} array The array to inspect
 * @param {ArrayLike<T2>} values1 The first array of values to exclude
 * @param {ArrayLike<T3>} values2 The second array of values to exclude
 * @param {ValueIteratee<T1 | T2 | T3>} iteratee The iteratee invoked per element
 * @returns {T1[]} Returns the new array of filtered values
 * @example
 * differenceBy([2.1, 1.2], [2.3], [1.4], Math.floor)
 * // => []
 */
declare function differenceBy<T1, T2, T3>(array: ArrayLike<T1> | null | undefined, values1: ArrayLike<T2>, values2: ArrayLike<T3>, iteratee: ValueIteratee<T1 | T2 | T3>): T1[];
/**
 * Creates an array of array values not included in the other given arrays using an iteratee function.
 *
 * @template T1, T2, T3, T4
 * @param {ArrayLike<T1> | null | undefined} array The array to inspect
 * @param {ArrayLike<T2>} values1 The first array of values to exclude
 * @param {ArrayLike<T3>} values2 The second array of values to exclude
 * @param {ArrayLike<T4>} values3 The third array of values to exclude
 * @param {ValueIteratee<T1 | T2 | T3 | T4>} iteratee The iteratee invoked per element
 * @returns {T1[]} Returns the new array of filtered values
 * @example
 * differenceBy([2.1, 1.2, 3.5], [2.3], [1.4], [3.2], Math.floor)
 * // => []
 */
declare function differenceBy<T1, T2, T3, T4>(array: ArrayLike<T1> | null | undefined, values1: ArrayLike<T2>, values2: ArrayLike<T3>, values3: ArrayLike<T4>, iteratee: ValueIteratee<T1 | T2 | T3 | T4>): T1[];
/**
 * Creates an array of array values not included in the other given arrays using an iteratee function.
 *
 * @template T1, T2, T3, T4, T5
 * @param {ArrayLike<T1> | null | undefined} array The array to inspect
 * @param {ArrayLike<T2>} values1 The first array of values to exclude
 * @param {ArrayLike<T3>} values2 The second array of values to exclude
 * @param {ArrayLike<T4>} values3 The third array of values to exclude
 * @param {ArrayLike<T5>} values4 The fourth array of values to exclude
 * @param {ValueIteratee<T1 | T2 | T3 | T4 | T5>} iteratee The iteratee invoked per element
 * @returns {T1[]} Returns the new array of filtered values
 * @example
 * differenceBy([2.1, 1.2, 3.5, 4.8], [2.3], [1.4], [3.2], [4.1], Math.floor)
 * // => []
 */
declare function differenceBy<T1, T2, T3, T4, T5>(array: ArrayLike<T1> | null | undefined, values1: ArrayLike<T2>, values2: ArrayLike<T3>, values3: ArrayLike<T4>, values4: ArrayLike<T5>, iteratee: ValueIteratee<T1 | T2 | T3 | T4 | T5>): T1[];
/**
 * Creates an array of array values not included in the other given arrays using an iteratee function.
 *
 * @template T1, T2, T3, T4, T5, T6
 * @param {ArrayLike<T1> | null | undefined} array The array to inspect
 * @param {ArrayLike<T2>} values1 The first array of values to exclude
 * @param {ArrayLike<T3>} values2 The second array of values to exclude
 * @param {ArrayLike<T4>} values3 The third array of values to exclude
 * @param {ArrayLike<T5>} values4 The fourth array of values to exclude
 * @param {ArrayLike<T6>} values5 The fifth array of values to exclude
 * @param {ValueIteratee<T1 | T2 | T3 | T4 | T5 | T6>} iteratee The iteratee invoked per element
 * @returns {T1[]} Returns the new array of filtered values
 * @example
 * differenceBy([2.1, 1.2, 3.5, 4.8, 5.3], [2.3], [1.4], [3.2], [4.1], [5.8], Math.floor)
 * // => []
 */
declare function differenceBy<T1, T2, T3, T4, T5, T6>(array: ArrayLike<T1> | null | undefined, values1: ArrayLike<T2>, values2: ArrayLike<T3>, values3: ArrayLike<T4>, values4: ArrayLike<T5>, values5: ArrayLike<T6>, iteratee: ValueIteratee<T1 | T2 | T3 | T4 | T5 | T6>): T1[];
/**
 * Creates an array of array values not included in the other given arrays using an iteratee function.
 *
 * @template T1, T2, T3, T4, T5, T6, T7
 * @param {ArrayLike<T1> | null | undefined} array The array to inspect
 * @param {ArrayLike<T2>} values1 The first array of values to exclude
 * @param {ArrayLike<T3>} values2 The second array of values to exclude
 * @param {ArrayLike<T4>} values3 The third array of values to exclude
 * @param {ArrayLike<T5>} values4 The fourth array of values to exclude
 * @param {ArrayLike<T6>} values5 The fifth array of values to exclude
 * @param {...(ArrayLike<T7> | ValueIteratee<T1 | T2 | T3 | T4 | T5 | T6 | T7>)[]} values Additional arrays of values to exclude and iteratee
 * @returns {T1[]} Returns the new array of filtered values
 * @example
 * differenceBy([2.1, 1.2, 3.5, 4.8, 5.3, 6.7], [2.3], [1.4], [3.2], [4.1], [5.8], [6.2], Math.floor)
 * // => []
 */
declare function differenceBy<T1, T2, T3, T4, T5, T6, T7>(array: ArrayLike<T1> | null | undefined, values1: ArrayLike<T2>, values2: ArrayLike<T3>, values3: ArrayLike<T4>, values4: ArrayLike<T5>, values5: ArrayLike<T6>, ...values: Array<ArrayLike<T7> | ValueIteratee<T1 | T2 | T3 | T4 | T5 | T6 | T7>>): T1[];
/**
 * Creates an array of array values not included in the other given arrays.
 *
 * @template T
 * @param {ArrayLike<T> | null | undefined} array The array to inspect
 * @param {...Array<ArrayLike<T>>} values The arrays of values to exclude
 * @returns {T[]} Returns the new array of filtered values
 * @example
 * differenceBy([2, 1], [2, 3])
 * // => [1]
 */
declare function differenceBy<T>(array: ArrayLike<T> | null | undefined, ...values: Array<ArrayLike<T>>): T[];

export { differenceBy };
