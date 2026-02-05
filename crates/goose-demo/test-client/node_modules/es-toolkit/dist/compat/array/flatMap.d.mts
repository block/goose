import { ListIterator } from '../_internal/ListIterator.mjs';
import { Many } from '../_internal/Many.mjs';
import { ObjectIterator } from '../_internal/ObjectIterator.mjs';

/**
 * Creates a flattened array of values by running each element in collection through iteratee and flattening the mapped results.
 *
 * @template T
 * @param {Record<string, Many<T>> | Record<number, Many<T>> | null | undefined} collection - The collection to iterate over.
 * @returns {T[]} Returns the new flattened array.
 *
 * @example
 * const obj = { a: [1, 2], b: [3, 4] };
 * flatMap(obj);
 * // => [1, 2, 3, 4]
 */
declare function flatMap<T>(collection: Record<string, Many<T>> | Record<number, Many<T>> | null | undefined): T[];
/**
 * Creates a flattened array of values by running each element in collection through iteratee and flattening the mapped results.
 *
 * @param {object | null | undefined} collection - The collection to iterate over.
 * @returns {any[]} Returns the new flattened array.
 *
 * @example
 * flatMap({ a: 1, b: 2 });
 * // => [1, 2]
 */
declare function flatMap(collection: object | null | undefined): any[];
/**
 * Creates a flattened array of values by running each element in collection through iteratee and flattening the mapped results.
 *
 * @template T, R
 * @param {ArrayLike<T> | null | undefined} collection - The collection to iterate over.
 * @param {ListIterator<T, Many<R>>} iteratee - The function invoked per iteration.
 * @returns {R[]} Returns the new flattened array.
 *
 * @example
 * function duplicate(n) {
 *   return [n, n];
 * }
 *
 * flatMap([1, 2], duplicate);
 * // => [1, 1, 2, 2]
 */
declare function flatMap<T, R>(collection: ArrayLike<T> | null | undefined, iteratee: ListIterator<T, Many<R>>): R[];
/**
 * Creates a flattened array of values by running each element in collection through iteratee and flattening the mapped results.
 *
 * @template T, R
 * @param {T | null | undefined} collection - The object to iterate over.
 * @param {ObjectIterator<T, Many<R>>} iteratee - The function invoked per iteration.
 * @returns {R[]} Returns the new flattened array.
 *
 * @example
 * const obj = { a: 1, b: 2 };
 * flatMap(obj, (value, key) => [key, value]);
 * // => ['a', 1, 'b', 2]
 */
declare function flatMap<T extends object, R>(collection: T | null | undefined, iteratee: ObjectIterator<T, Many<R>>): R[];
/**
 * Creates a flattened array of values by running each element in collection through iteratee and flattening the mapped results.
 *
 * @param {object | null | undefined} collection - The collection to iterate over.
 * @param {string} iteratee - The property name to use as iteratee.
 * @returns {any[]} Returns the new flattened array.
 *
 * @example
 * const users = [
 *   { user: 'barney', hobbies: ['hiking', 'coding'] },
 *   { user: 'fred', hobbies: ['reading'] }
 * ];
 * flatMap(users, 'hobbies');
 * // => ['hiking', 'coding', 'reading']
 */
declare function flatMap(collection: object | null | undefined, iteratee: string): any[];
/**
 * Creates a flattened array of values by running each element in collection through iteratee and flattening the mapped results.
 *
 * @param {object | null | undefined} collection - The collection to iterate over.
 * @param {object} iteratee - The object properties to match.
 * @returns {boolean[]} Returns the new flattened array.
 *
 * @example
 * const users = [
 *   { user: 'barney', age: 36, active: true },
 *   { user: 'fred', age: 40, active: false }
 * ];
 * flatMap(users, { active: false });
 * // => [false]
 */
declare function flatMap(collection: object | null | undefined, iteratee: object): boolean[];

export { flatMap };
