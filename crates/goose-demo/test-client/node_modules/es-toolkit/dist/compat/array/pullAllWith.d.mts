import { MutableList } from '../_internal/MutableList.d.mjs';
import { RejectReadonly } from '../_internal/RejectReadonly.d.mjs';

/**
 * This method is like `_.pullAll` except that it accepts `comparator` which is
 * invoked to compare elements of array to values. The comparator is invoked with
 * two arguments: (arrVal, othVal).
 *
 * **Note:** Unlike `_.differenceWith`, this method mutates `array`.
 *
 * @template T
 * @param {T[]} array - The array to modify.
 * @param {ArrayLike<T>} [values] - The values to remove.
 * @param {(a: T, b: T) => boolean} [comparator] - The comparator invoked per element.
 * @returns {T[]} Returns `array`.
 *
 * @example
 * var array = [{ 'x': 1, 'y': 2 }, { 'x': 3, 'y': 4 }, { 'x': 5, 'y': 6 }];
 *
 * pullAllWith(array, [{ 'x': 3, 'y': 4 }], _.isEqual);
 * console.log(array);
 * // => [{ 'x': 1, 'y': 2 }, { 'x': 5, 'y': 6 }]
 */
declare function pullAllWith<T>(array: T[], values?: ArrayLike<T>, comparator?: (a: T, b: T) => boolean): T[];
/**
 * This method is like `_.pullAll` except that it accepts `comparator` which is
 * invoked to compare elements of array to values. The comparator is invoked with
 * two arguments: (arrVal, othVal).
 *
 * **Note:** Unlike `_.differenceWith`, this method mutates `array`.
 *
 * @template L
 * @param {RejectReadonly<L>} array - The array to modify.
 * @param {List<L[0]>} [values] - The values to remove.
 * @param {Comparator<L[0]>} [comparator] - The comparator invoked per element.
 * @returns {L} Returns `array`.
 *
 * @example
 * var array = [{ 'x': 1, 'y': 2 }, { 'x': 3, 'y': 4 }, { 'x': 5, 'y': 6 }];
 *
 * pullAllWith(array, [{ 'x': 3, 'y': 4 }], _.isEqual);
 * console.log(array);
 * // => [{ 'x': 1, 'y': 2 }, { 'x': 5, 'y': 6 }]
 */
declare function pullAllWith<L extends MutableList<any>>(array: RejectReadonly<L>, values?: ArrayLike<L[0]>, comparator?: (a: L[0], b: L[0]) => boolean): L;
/**
 * This method is like `_.pullAll` except that it accepts `comparator` which is
 * invoked to compare elements of array to values. The comparator is invoked with
 * two arguments: (arrVal, othVal).
 *
 * **Note:** Unlike `_.differenceWith`, this method mutates `array`.
 *
 * @template T, U
 * @param {T[]} array - The array to modify.
 * @param {ArrayLike<U>} values - The values to remove.
 * @param {(a: T, b: U) => boolean} comparator - The comparator invoked per element.
 * @returns {T[]} Returns `array`.
 *
 * @example
 * var array = [{ 'x': 1, 'y': 2 }, { 'x': 3, 'y': 4 }, { 'x': 5, 'y': 6 }];
 *
 * pullAllWith(array, [{ 'x': 3, 'y': 4 }], _.isEqual);
 * console.log(array);
 * // => [{ 'x': 1, 'y': 2 }, { 'x': 5, 'y': 6 }]
 */
declare function pullAllWith<T, U>(array: T[], values: ArrayLike<U>, comparator: (a: T, b: U) => boolean): T[];
/**
 * This method is like `_.pullAll` except that it accepts `comparator` which is
 * invoked to compare elements of array to values. The comparator is invoked with
 * two arguments: (arrVal, othVal).
 *
 * **Note:** Unlike `_.differenceWith`, this method mutates `array`.
 *
 * @template L1, L2
 * @param {RejectReadonly<L1>} array - The array to modify.
 * @param {List<L2>} values - The values to remove.
 * @param {Comparator2<L1[0], L2>} comparator - The comparator invoked per element.
 * @returns {L1} Returns `array`.
 *
 * @example
 * var array = [{ 'x': 1, 'y': 2 }, { 'x': 3, 'y': 4 }, { 'x': 5, 'y': 6 }];
 *
 * pullAllWith(array, [{ 'x': 3, 'y': 4 }], _.isEqual);
 * console.log(array);
 * // => [{ 'x': 1, 'y': 2 }, { 'x': 5, 'y': 6 }]
 */
declare function pullAllWith<L1 extends MutableList<any>, L2>(array: RejectReadonly<L1>, values: ArrayLike<L2>, comparator: (a: L1[0], b: L2) => boolean): L1;

export { pullAllWith };
