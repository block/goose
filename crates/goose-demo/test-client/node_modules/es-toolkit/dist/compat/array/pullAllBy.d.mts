import { MutableList } from '../_internal/MutableList.d.mjs';
import { RejectReadonly } from '../_internal/RejectReadonly.d.mjs';
import { ValueIteratee } from '../_internal/ValueIteratee.mjs';

/**
 * Removes all specified values from an array using an iteratee function.
 *
 * This function changes `arr` in place.
 * If you want to remove values without modifying the original array, use `differenceBy`.
 *
 * @template T
 * @param {T[]} array - The array to modify.
 * @param {ArrayLike<T>} [values] - The values to remove.
 * @param {((value: T) => unknown) | PropertyKey | [PropertyKey, any] | Partial<T>} [iteratee] - The iteratee invoked per element.
 * @returns {T[]} Returns `array`.
 *
 * @example
 * var array = [{ 'x': 1 }, { 'x': 2 }, { 'x': 3 }, { 'x': 1 }];
 *
 * pullAllBy(array, [{ 'x': 1 }, { 'x': 3 }], 'x');
 * console.log(array);
 * // => [{ 'x': 2 }]
 */
declare function pullAllBy<T>(array: T[], values?: ArrayLike<T>, iteratee?: ValueIteratee<T>): T[];
/**
 * Removes all specified values from an array using an iteratee function.
 *
 * This function changes `arr` in place.
 * If you want to remove values without modifying the original array, use `differenceBy`.
 *
 * @template L
 * @param {RejectReadonly<L>} array - The array to modify.
 * @param {ArrayLike<L[0]>} [values] - The values to remove.
 * @param {ValueIteratee<L[0]>} [iteratee] - The iteratee invoked per element.
 * @returns {L} Returns `array`.
 *
 * @example
 * var array = [{ 'x': 1 }, { 'x': 2 }, { 'x': 3 }, { 'x': 1 }];
 *
 * pullAllBy(array, [{ 'x': 1 }, { 'x': 3 }], 'x');
 * console.log(array);
 * // => [{ 'x': 2 }]
 */
declare function pullAllBy<L extends MutableList<any>>(array: RejectReadonly<L>, values?: ArrayLike<L[0]>, iteratee?: ValueIteratee<L[0]>): L;
/**
 * Removes all specified values from an array using an iteratee function.
 *
 * This function changes `arr` in place.
 * If you want to remove values without modifying the original array, use `differenceBy`.
 *
 * @template T, U
 * @param {T[]} array - The array to modify.
 * @param {ArrayLike<U>} values - The values to remove.
 * @param {((value: T | U) => unknown) | PropertyKey | [PropertyKey, any] | Partial<T | U>} iteratee - The iteratee invoked per element.
 * @returns {T[]} Returns `array`.
 *
 * @example
 * var array = [{ 'x': 1 }, { 'x': 2 }, { 'x': 3 }, { 'x': 1 }];
 *
 * pullAllBy(array, [{ 'x': 1 }, { 'x': 3 }], 'x');
 * console.log(array);
 * // => [{ 'x': 2 }]
 */
declare function pullAllBy<T, U>(array: T[], values: ArrayLike<U>, iteratee: ValueIteratee<T | U>): T[];
/**
 * Removes all specified values from an array using an iteratee function.
 *
 * This function changes `arr` in place.
 * If you want to remove values without modifying the original array, use `differenceBy`.
 *
 * @template L, U
 * @param {L} array - The array to modify.
 * @param {ArrayLike<U>} values - The values to remove.
 * @param {((value: L[0] | U) => unknown) | PropertyKey | [PropertyKey, any] | Partial<L[0] | U>} iteratee - The iteratee invoked per element.
 * @returns {L} Returns `array`.
 *
 * @example
 * var array = [{ 'x': 1 }, { 'x': 2 }, { 'x': 3 }, { 'x': 1 }];
 *
 * pullAllBy(array, [{ 'x': 1 }, { 'x': 3 }], 'x');
 * console.log(array);
 * // => [{ 'x': 2 }]
 */
declare function pullAllBy<L extends ArrayLike<any>, U>(array: L extends readonly any[] ? never : L, values: ArrayLike<U>, iteratee: ValueIteratee<L[0] | U>): L;

export { pullAllBy };
