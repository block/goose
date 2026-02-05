import { ListIteratee } from '../_internal/ListIteratee.mjs';
import { Many } from '../_internal/Many.mjs';
import { ObjectIteratee } from '../_internal/ObjectIteratee.mjs';

/**
 * Sorts an array of objects based on multiple properties and their corresponding order directions.
 *
 * This function takes an array of objects, an array of criteria to sort by.
 * It returns the ascending sorted array, ordering by each key.
 * If values for a key are equal, it moves to the next key to determine the order.
 *
 * @template T - The type of elements in the array.
 * @param {ArrayLike<T> | object | null | undefined} collection - The array of objects to be sorted.
 * @param {Array<Array<Criterion<T> | Criterion<T>>>} criteria - An array of criteria (property names or property paths or custom key functions) to sort by.
 * @returns {T[]} - The ascending sorted array.
 *
 * @example
 * // Sort an array of objects by 'user' in ascending order and 'age' in descending order.
 * const users = [
 *   { user: 'fred', age: 48 },
 *   { user: 'barney', age: 34 },
 *   { user: 'fred', age: 40 },
 *   { user: 'barney', age: 36 },
 * ];
 * const result = sortBy(users, ['user', (item) => item.age])
 * // result will be:
 * // [
 * //   { user: 'barney', age: 34 },
 * //   { user: 'barney', age: 36 },
 * //   { user: 'fred', age: 40 },
 * //   { user: 'fred', age: 48 },
 * // ]
 */
/**
 * Creates an array of elements, sorted in ascending order by the results of running each element in a collection thru each iteratee.
 *
 * @template T
 * @param {ArrayLike<T> | null | undefined} collection - The collection to iterate over.
 * @param {...Array<T | readonly T[] | ListIteratee<T>>} iteratees - The iteratees to sort by.
 * @returns {T[]} Returns the new sorted array.
 *
 * @example
 * const users = [
 *   { 'user': 'fred',   'age': 48 },
 *   { 'user': 'barney', 'age': 36 },
 *   { 'user': 'fred',   'age': 42 },
 *   { 'user': 'barney', 'age': 34 }
 * ];
 *
 * sortBy(users, [function(o) { return o.user; }]);
 * // => objects for [['barney', 36], ['barney', 34], ['fred', 48], ['fred', 42]]
 */
declare function sortBy<T>(collection: ArrayLike<T> | null | undefined, ...iteratees: Array<Many<ListIteratee<T>>>): T[];
/**
 * Creates an array of elements, sorted in ascending order by the results of running each element in a collection thru each iteratee.
 *
 * @template T
 * @param {T | null | undefined} collection - The object to iterate over.
 * @param {...Array<T[keyof T] | readonly Array<T[keyof T]> | ObjectIteratee<T>>} iteratees - The iteratees to sort by.
 * @returns {Array<T[keyof T]>} Returns the new sorted array.
 *
 * @example
 * const users = {
 *   'a': { 'user': 'fred',   'age': 48 },
 *   'b': { 'user': 'barney', 'age': 36 }
 * };
 *
 * sortBy(users, [function(o) { return o.user; }]);
 * // => [{ 'user': 'barney', 'age': 36 }, { 'user': 'fred', 'age': 48 }]
 */
declare function sortBy<T extends object>(collection: T | null | undefined, ...iteratees: Array<Many<ObjectIteratee<T>>>): Array<T[keyof T]>;

export { sortBy };
