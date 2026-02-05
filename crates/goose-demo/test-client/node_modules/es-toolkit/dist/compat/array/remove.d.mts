import { ListIteratee } from '../_internal/ListIteratee.mjs';
import { MutableList } from '../_internal/MutableList.d.mjs';
import { RejectReadonly } from '../_internal/RejectReadonly.d.mjs';

/**
 * Removes all elements from array that predicate returns truthy for and returns an array of the removed elements.
 *
 * @template L
 * @param {RejectReadonly<L>} array - The array to modify.
 * @param {ListIteratee<L[0]>} [predicate] - The function invoked per iteration.
 * @returns {Array<L[0]>} Returns the new array of removed elements.
 *
 * @example
 * const array = [1, 2, 3, 4];
 * const evens = remove(array, n => n % 2 === 0);
 * console.log(array); // => [1, 3]
 * console.log(evens); // => [2, 4]
 */
declare function remove<L extends MutableList<any>>(array: RejectReadonly<L>, predicate?: ListIteratee<L[0]>): Array<L[0]>;

export { remove };
