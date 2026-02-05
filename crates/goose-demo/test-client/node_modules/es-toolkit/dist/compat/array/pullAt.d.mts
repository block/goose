import { Many } from '../_internal/Many.mjs';
import { MutableList } from '../_internal/MutableList.d.mjs';
import { RejectReadonly } from '../_internal/RejectReadonly.d.mjs';

/**
 * Removes elements from array corresponding to the given indexes and returns an array of the removed elements.
 * Indexes may be specified as an array of indexes or as individual arguments.
 *
 * **Note:** Unlike `_.at`, this method mutates `array`.
 *
 * @template T
 * @param {T[]} array - The array to modify.
 * @param {...Array<number | number[]>} indexes - The indexes of elements to remove, specified as individual indexes or arrays of indexes.
 * @returns {T[]} Returns the new array of removed elements.
 *
 * @example
 * var array = [5, 10, 15, 20];
 * var evens = pullAt(array, 1, 3);
 *
 * console.log(array);
 * // => [5, 15]
 *
 * console.log(evens);
 * // => [10, 20]
 */
declare function pullAt<T>(array: T[], ...indexes: Array<Many<number>>): T[];
/**
 * Removes elements from array corresponding to the given indexes and returns an array of the removed elements.
 * Indexes may be specified as an array of indexes or as individual arguments.
 *
 * **Note:** Unlike `_.at`, this method mutates `array`.
 *
 * @template L
 * @param {L} array - The array to modify.
 * @param {...Array<number | number[]>} indexes - The indexes of elements to remove, specified as individual indexes or arrays of indexes.
 * @returns {L} Returns the new array of removed elements.
 *
 * @example
 * var array = [5, 10, 15, 20];
 * var evens = pullAt(array, 1, 3);
 *
 * console.log(array);
 * // => [5, 15]
 *
 * console.log(evens);
 * // => [10, 20]
 */
declare function pullAt<L extends MutableList<any>>(array: RejectReadonly<L>, ...indexes: Array<Many<number>>): L;

export { pullAt };
