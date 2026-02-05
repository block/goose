import { ListIterateeCustom } from '../_internal/ListIterateeCustom.js';

/**
 * Finds the index of the last element in the array that satisfies the predicate.
 *
 * @template T
 * @param {ArrayLike<T> | null | undefined} array - The array to search through.
 * @param {ListIterateeCustom<T, boolean>} [predicate] - The predicate function, partial object, property-value pair, or property name.
 * @param {number} [fromIndex] - The index to start searching from.
 * @returns {number} The index of the last matching element, or -1 if not found.
 *
 * @example
 * const users = [
 *   { user: 'barney', active: true },
 *   { user: 'fred', active: false },
 *   { user: 'pebbles', active: false }
 * ];
 *
 * findLastIndex(users, o => o.user === 'pebbles');
 * // => 2
 *
 * findLastIndex(users, { user: 'barney', active: true });
 * // => 0
 *
 * findLastIndex(users, ['active', false]);
 * // => 2
 *
 * findLastIndex(users, 'active');
 * // => 0
 */
declare function findLastIndex<T>(array: ArrayLike<T> | null | undefined, predicate?: ListIterateeCustom<T, boolean>, fromIndex?: number): number;

export { findLastIndex };
