import { ListIteratee } from '../_internal/ListIteratee.js';

/**
 * Creates a slice of array excluding elements dropped from the end until predicate returns falsey.
 * The predicate is invoked with three arguments: (value, index, array).
 *
 * @template T - The type of elements in the array.
 * @param {ArrayLike<T> | null | undefined} array - The array to query.
 * @param {ListIteratee<T>} [predicate] - The function invoked per iteration.
 * @returns {T[]} Returns the slice of array.
 * @example
 *
 * const users = [
 *   { user: 'barney', active: true },
 *   { user: 'fred', active: false },
 *   { user: 'pebbles', active: false }
 * ];
 *
 * // Using function predicate
 * dropRightWhile(users, user => !user.active);
 * // => [{ user: 'barney', active: true }]
 *
 * // Using matches shorthand
 * dropRightWhile(users, { user: 'pebbles', active: false });
 * // => [{ user: 'barney', active: true }, { user: 'fred', active: false }]
 *
 * // Using matchesProperty shorthand
 * dropRightWhile(users, ['active', false]);
 * // => [{ user: 'barney', active: true }]
 *
 * // Using property shorthand
 * dropRightWhile(users, 'active');
 * // => [{ user: 'barney', active: true }]
 */
declare function dropRightWhile<T>(array: ArrayLike<T> | null | undefined, predicate?: ListIteratee<T>): T[];

export { dropRightWhile };
