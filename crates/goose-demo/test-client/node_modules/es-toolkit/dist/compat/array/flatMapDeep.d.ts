import { ListIterator } from '../_internal/ListIterator.js';
import { ListOfRecursiveArraysOrValues } from '../_internal/ListOfRecursiveArraysOrValues.js';
import { ObjectIterator } from '../_internal/ObjectIterator.js';

/**
 * Creates a flattened array of values by running each element through iteratee and recursively flattening the mapped results.
 *
 * @template T
 * @param {Record<string, RecursiveArray<T> | T> | Record<number, RecursiveArray<T> | T> | null | undefined} collection - The collection to iterate over.
 * @returns {T[]} Returns the new deeply flattened array.
 *
 * @example
 * const obj = { a: [[1, 2]], b: [[[3]]] };
 * flatMapDeep(obj);
 * // => [1, 2, 3]
 */
declare function flatMapDeep<T>(collection: Record<string, ListOfRecursiveArraysOrValues<T> | T> | Record<number, ListOfRecursiveArraysOrValues<T> | T> | null | undefined): T[];
/**
 * Creates a flattened array of values by running each element through iteratee and recursively flattening the mapped results.
 *
 * @template T, R
 * @param {ArrayLike<T> | null | undefined} collection - The collection to iterate over.
 * @param {ListIterator<T, RecursiveArray<R> | R>} iteratee - The function invoked per iteration.
 * @returns {R[]} Returns the new deeply flattened array.
 *
 * @example
 * function duplicate(n) {
 *   return [[[n, n]]];
 * }
 *
 * flatMapDeep([1, 2], duplicate);
 * // => [1, 1, 2, 2]
 */
declare function flatMapDeep<T, R>(collection: ArrayLike<T> | null | undefined, iteratee: ListIterator<T, ListOfRecursiveArraysOrValues<R> | R>): R[];
/**
 * Creates a flattened array of values by running each element through iteratee and recursively flattening the mapped results.
 *
 * @template T, R
 * @param {T | null | undefined} collection - The object to iterate over.
 * @param {ObjectIterator<T, RecursiveArray<R> | R>} iteratee - The function invoked per iteration.
 * @returns {R[]} Returns the new deeply flattened array.
 *
 * @example
 * const obj = { a: 1, b: 2 };
 * flatMapDeep(obj, (value, key) => [[[key, value]]]);
 * // => ['a', 1, 'b', 2]
 */
declare function flatMapDeep<T extends object, R>(collection: T | null | undefined, iteratee: ObjectIterator<T, ListOfRecursiveArraysOrValues<R> | R>): R[];
/**
 * Creates a flattened array of values by running each element through iteratee and recursively flattening the mapped results.
 *
 * @param {object | null | undefined} collection - The collection to iterate over.
 * @param {string} iteratee - The property name to use as iteratee.
 * @returns {any[]} Returns the new deeply flattened array.
 *
 * @example
 * const users = [
 *   { user: 'barney', hobbies: [['hiking', 'coding']] },
 *   { user: 'fred', hobbies: [['reading']] }
 * ];
 * flatMapDeep(users, 'hobbies');
 * // => ['hiking', 'coding', 'reading']
 */
declare function flatMapDeep(collection: object | null | undefined, iteratee: string): any[];
/**
 * Creates a flattened array of values by running each element through iteratee and recursively flattening the mapped results.
 *
 * @param {object | null | undefined} collection - The collection to iterate over.
 * @param {object} iteratee - The object properties to match.
 * @returns {boolean[]} Returns the new deeply flattened array.
 *
 * @example
 * const users = [
 *   { user: 'barney', active: [true, false] },
 *   { user: 'fred', active: [false] }
 * ];
 * flatMapDeep(users, { active: [false] });
 * // => [false]
 */
declare function flatMapDeep(collection: object | null | undefined, iteratee: object): boolean[];

export { flatMapDeep };
