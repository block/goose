import { ListIteratee } from '../_internal/ListIteratee.mjs';

/**
 * Creates a slice of array with elements taken from the end. Elements are taken until predicate
 * returns falsey. The predicate is invoked with three arguments: (value, index, array).
 *
 * @template T
 * @param {ArrayLike<T> | null | undefined} array - The array to query.
 * @param {ListIteratee<T>} [predicate] - The function invoked per iteration.
 * @returns {T[]} Returns the slice of array.
 *
 * @example
 * const users = [
 *   { 'user': 'barney',  'active': true },
 *   { 'user': 'fred',    'active': false },
 *   { 'user': 'pebbles', 'active': false }
 * ];
 *
 * takeRightWhile(users, function(o) { return !o.active; });
 * // => objects for ['fred', 'pebbles']
 *
 * @example
 * takeRightWhile(users, { 'user': 'pebbles', 'active': false });
 * // => objects for ['pebbles']
 *
 * @example
 * takeRightWhile(users, ['active', false]);
 * // => objects for ['fred', 'pebbles']
 *
 * @example
 * takeRightWhile(users, 'active');
 * // => []
 */
declare function takeRightWhile<T>(array: ArrayLike<T> | null | undefined, predicate?: ListIteratee<T>): T[];

export { takeRightWhile };
