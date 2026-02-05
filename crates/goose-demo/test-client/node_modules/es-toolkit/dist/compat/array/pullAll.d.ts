import { MutableList } from '../_internal/MutableList.d.js';
import { RejectReadonly } from '../_internal/RejectReadonly.d.js';

/**
 * This method is like `_.pull` except that it accepts an array of values to remove.
 *
 * **Note:** Unlike `_.difference`, this method mutates `array`.
 *
 * @template T
 * @param {T[]} array - The array to modify.
 * @param {ArrayLike<T>} [values] - The values to remove.
 * @returns {T[]} Returns `array`.
 *
 * @example
 * var array = [1, 2, 3, 1, 2, 3];
 *
 * pullAll(array, [2, 3]);
 * console.log(array);
 * // => [1, 1]
 */
declare function pullAll<T>(array: T[], values?: ArrayLike<T>): T[];
/**
 * This method is like `_.pull` except that it accepts an array of values to remove.
 *
 * **Note:** Unlike `_.difference`, this method mutates `array`.
 *
 * @template L
 * @param {RejectReadonly<L>} array - The array to modify.
 * @param {List<L[0]>} [values] - The values to remove.
 * @returns {L} Returns `array`.
 *
 * @example
 * var array = [1, 2, 3, 1, 2, 3];
 *
 * pullAll(array, [2, 3]);
 * console.log(array);
 * // => [1, 1]
 */
declare function pullAll<L extends MutableList<any>>(array: RejectReadonly<L>, values?: ArrayLike<L[0]>): L;

export { pullAll };
