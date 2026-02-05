import { ListIteratee } from '../_internal/ListIteratee.mjs';
import { ListIterator } from '../_internal/ListIterator.mjs';
import { Many } from '../_internal/Many.mjs';
import { ObjectIteratee } from '../_internal/ObjectIteratee.mjs';
import { ObjectIterator } from '../_internal/ObjectIterator.mjs';

/**
 * Sorts an array of elements based on multiple iteratee functions and their corresponding order directions.
 *
 * @template T The type of elements in the array
 * @param {ArrayLike<T> | null | undefined} collection The array to sort
 * @param {Many<ListIterator<T, unknown>>} iteratees The iteratee functions to sort by
 * @param {Many<boolean | 'asc' | 'desc'>} orders The sort orders
 * @returns {T[]} Returns the new sorted array
 * @example
 * const users = [
 *   { name: 'fred', age: 48 },
 *   { name: 'barney', age: 34 }
 * ];
 *
 * // Sort by age in ascending order
 * orderBy(users, [(user) => user.age], ['asc']);
 * // => [{ name: 'barney', age: 34 }, { name: 'fred', age: 48 }]
 */
declare function orderBy<T>(collection: ArrayLike<T> | null | undefined, iteratees?: Many<ListIterator<T, unknown>>, orders?: Many<boolean | 'asc' | 'desc'>): T[];
/**
 * Sorts an array of elements based on multiple property names/paths and their corresponding order directions.
 *
 * @template T The type of elements in the array
 * @param {ArrayLike<T> | null | undefined} collection The array to sort
 * @param {Many<ListIteratee<T>>} iteratees The property names/paths to sort by
 * @param {Many<boolean | 'asc' | 'desc'>} orders The sort orders
 * @returns {T[]} Returns the new sorted array
 * @example
 * const users = [
 *   { name: 'fred', age: 48 },
 *   { name: 'barney', age: 34 }
 * ];
 *
 * // Sort by name in ascending order
 * orderBy(users, ['name'], ['asc']);
 * // => [{ name: 'barney', age: 34 }, { name: 'fred', age: 48 }]
 */
declare function orderBy<T>(collection: ArrayLike<T> | null | undefined, iteratees?: Many<ListIteratee<T>>, orders?: Many<boolean | 'asc' | 'desc'>): T[];
/**
 * Sorts an object's values based on multiple iteratee functions and their corresponding order directions.
 *
 * @template T The object type
 * @param {T | null | undefined} collection The object to sort values from
 * @param {Many<ObjectIterator<T, unknown>>} iteratees The iteratee functions to sort by
 * @param {Many<boolean | 'asc' | 'desc'>} orders The sort orders
 * @returns {Array<T[keyof T]>} Returns the new sorted array
 * @example
 * const obj = {
 *   a: { name: 'fred', age: 48 },
 *   b: { name: 'barney', age: 34 }
 * };
 *
 * // Sort by age in ascending order
 * orderBy(obj, [(user) => user.age], ['asc']);
 * // => [{ name: 'barney', age: 34 }, { name: 'fred', age: 48 }]
 */
declare function orderBy<T extends object>(collection: T | null | undefined, iteratees?: Many<ObjectIterator<T, unknown>>, orders?: Many<boolean | 'asc' | 'desc'>): Array<T[keyof T]>;
/**
 * Sorts an object's values based on multiple property names/paths and their corresponding order directions.
 *
 * @template T The object type
 * @param {T | null | undefined} collection The object to sort values from
 * @param {Many<ObjectIteratee<T>>} iteratees The property names/paths to sort by
 * @param {Many<boolean | 'asc' | 'desc'>} orders The sort orders
 * @returns {Array<T[keyof T]>} Returns the new sorted array
 * @example
 * const obj = {
 *   a: { name: 'fred', age: 48 },
 *   b: { name: 'barney', age: 34 }
 * };
 *
 * // Sort by name in ascending order
 * orderBy(obj, ['name'], ['asc']);
 * // => [{ name: 'barney', age: 34 }, { name: 'fred', age: 48 }]
 */
declare function orderBy<T extends object>(collection: T | null | undefined, iteratees?: Many<ObjectIteratee<T>>, orders?: Many<boolean | 'asc' | 'desc'>): Array<T[keyof T]>;

export { orderBy };
