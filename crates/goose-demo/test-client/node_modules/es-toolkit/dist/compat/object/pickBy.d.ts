import { ValueKeyIteratee } from '../_internal/ValueKeyIteratee.js';
import { ValueKeyIterateeTypeGuard } from '../_internal/ValueKeyIterateeTypeGuard.js';

/**
 * Creates a new object composed of the properties that satisfy the predicate function.
 *
 * @template T - The type of object values.
 * @template S - The type of filtered values.
 * @param {Record<string, T> | null | undefined} object - The source object.
 * @param {ValueKeyIterateeTypeGuard<T, S>} predicate - The function invoked per property.
 * @returns {Record<string, S>} Returns the new filtered object.
 *
 * @example
 * const users = {
 *   'fred': { 'user': 'fred', 'age': 40 },
 *   'pebbles': { 'user': 'pebbles', 'age': 1 }
 * };
 * pickBy(users, ({ age }) => age < 40);
 * // => { 'pebbles': { 'user': 'pebbles', 'age': 1 } }
 */
declare function pickBy<T, S extends T>(object: Record<string, T> | null | undefined, predicate: ValueKeyIterateeTypeGuard<T, S>): Record<string, S>;
/**
 * Creates a new object composed of the properties that satisfy the predicate function.
 *
 * @template T - The type of object values.
 * @template S - The type of filtered values.
 * @param {Record<number, T> | null | undefined} object - The source object.
 * @param {ValueKeyIterateeTypeGuard<T, S>} predicate - The function invoked per property.
 * @returns {Record<number, S>} Returns the new filtered object.
 *
 * @example
 * const array = [1, 2, 3, 4];
 * pickBy(array, (value) => value % 2 === 0);
 * // => { 1: 2, 3: 4 }
 */
declare function pickBy<T, S extends T>(object: Record<number, T> | null | undefined, predicate: ValueKeyIterateeTypeGuard<T, S>): Record<number, S>;
/**
 * Creates a new object composed of the properties that satisfy the predicate function.
 *
 * @template T - The type of object values.
 * @param {Record<string, T> | null | undefined} object - The source object.
 * @param {ValueKeyIteratee<T>} [predicate] - The function invoked per property.
 * @returns {Record<string, T>} Returns the new filtered object.
 *
 * @example
 * const object = { 'a': 1, 'b': '2', 'c': 3 };
 * pickBy(object, (value) => typeof value === 'string');
 * // => { 'b': '2' }
 */
declare function pickBy<T>(object: Record<string, T> | null | undefined, predicate?: ValueKeyIteratee<T>): Record<string, T>;
/**
 * Creates a new object composed of the properties that satisfy the predicate function.
 *
 * @template T - The type of object values.
 * @param {Record<number, T> | null | undefined} object - The source object.
 * @param {ValueKeyIteratee<T>} [predicate] - The function invoked per property.
 * @returns {Record<number, T>} Returns the new filtered object.
 *
 * @example
 * const array = [1, 2, 3, 4];
 * pickBy(array, (value) => value > 2);
 * // => { 2: 3, 3: 4 }
 */
declare function pickBy<T>(object: Record<number, T> | null | undefined, predicate?: ValueKeyIteratee<T>): Record<number, T>;
/**
 * Creates a new object composed of the properties that satisfy the predicate function.
 *
 * @template T - The type of object.
 * @param {T | null | undefined} object - The source object.
 * @param {ValueKeyIteratee<T[keyof T]>} [predicate] - The function invoked per property.
 * @returns {Partial<T>} Returns the new filtered object.
 *
 * @example
 * const object = { 'a': 1, 'b': '2', 'c': 3 };
 * pickBy(object, (value) => typeof value === 'string');
 * // => { 'b': '2' }
 */
declare function pickBy<T extends object>(object: T | null | undefined, predicate?: ValueKeyIteratee<T[keyof T]>): Partial<T>;

export { pickBy };
