import { ListIteratee } from '../_internal/ListIteratee.mjs';

/**
 * Creates a slice of array with elements taken from the beginning. Elements are taken until predicate
 * returns falsey. The predicate is invoked with three arguments: (value, index, array).
 *
 * @template T
 * @param {ArrayLike<T> | null | undefined} array - The array to query.
 * @param {ListIteratee<T>} [predicate] - The function invoked per iteration.
 * @returns {T[]} Returns the slice of array.
 *
 * @example
 * const users = [
 *   { 'user': 'barney',  'active': false },
 *   { 'user': 'fred',    'active': false },
 *   { 'user': 'pebbles', 'active': true }
 * ];
 *
 * takeWhile(users, function(o) { return !o.active; });
 * // => objects for ['barney', 'fred']
 *
 * @example
 * takeWhile(users, { 'user': 'barney', 'active': false });
 * // => objects for ['barney']
 *
 * @example
 * takeWhile(users, ['active', false]);
 * // => objects for ['barney', 'fred']
 *
 * @example
 * takeWhile(users, 'active');
 * // => []
 */
declare function takeWhile<T>(array: ArrayLike<T> | null | undefined, predicate?: ListIteratee<T>): T[];

export { takeWhile };
