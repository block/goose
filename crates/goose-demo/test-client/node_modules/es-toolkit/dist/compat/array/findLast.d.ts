import { ListIterateeCustom } from '../_internal/ListIterateeCustom.js';
import { ListIteratorTypeGuard } from '../_internal/ListIteratorTypeGuard.js';
import { ObjectIterateeCustom } from '../_internal/ObjectIteratee.js';
import { ObjectIteratorTypeGuard } from '../_internal/ObjectIterator.js';

/**
 * Finds the last element in a collection that satisfies the predicate.
 *
 * @template T, S
 * @param {ArrayLike<T> | null | undefined} collection - The collection to search.
 * @param {ListIteratorTypeGuard<T, S>} predicate - The predicate function with type guard.
 * @param {number} [fromIndex] - The index to start searching from.
 * @returns {S | undefined} The last element that satisfies the predicate.
 *
 * @example
 * const users = [{ user: 'barney', age: 36 }, { user: 'fred', age: 40 }, { user: 'pebbles', age: 18 }];
 * findLast(users, (o): o is { user: string; age: number } => o.age < 40);
 * // => { user: 'pebbles', age: 18 }
 */
declare function findLast<T, S extends T>(collection: ArrayLike<T> | null | undefined, predicate: ListIteratorTypeGuard<T, S>, fromIndex?: number): S | undefined;
/**
 * Finds the last element in a collection that satisfies the predicate.
 *
 * @template T
 * @param {ArrayLike<T> | null | undefined} collection - The collection to search.
 * @param {ListIterateeCustom<T, boolean>} [predicate] - The predicate function, partial object, property-value pair, or property name.
 * @param {number} [fromIndex] - The index to start searching from.
 * @returns {T | undefined} The last element that satisfies the predicate.
 *
 * @example
 * const users = [{ user: 'barney', age: 36 }, { user: 'fred', age: 40 }, { user: 'pebbles', age: 18 }];
 * findLast(users, o => o.age < 40);
 * // => { user: 'pebbles', age: 18 }
 *
 * findLast(users, { age: 36 });
 * // => { user: 'barney', age: 36 }
 *
 * findLast(users, ['age', 18]);
 * // => { user: 'pebbles', age: 18 }
 *
 * findLast(users, 'age');
 * // => { user: 'fred', age: 40 }
 */
declare function findLast<T>(collection: ArrayLike<T> | null | undefined, predicate?: ListIterateeCustom<T, boolean>, fromIndex?: number): T | undefined;
/**
 * Finds the last element in an object that satisfies the predicate with type guard.
 *
 * @template T, S
 * @param {T | null | undefined} collection - The object to search.
 * @param {ObjectIteratorTypeGuard<T, S>} predicate - The predicate function with type guard.
 * @param {number} [fromIndex] - The index to start searching from.
 * @returns {S | undefined} The last element that satisfies the predicate.
 *
 * @example
 * const obj = { a: 1, b: 'hello', c: 3 };
 * findLast(obj, (value): value is string => typeof value === 'string');
 * // => 'hello'
 */
declare function findLast<T extends object, S extends T[keyof T]>(collection: T | null | undefined, predicate: ObjectIteratorTypeGuard<T, S>, fromIndex?: number): S | undefined;
/**
 * Finds the last element in an object that satisfies the predicate.
 *
 * @template T
 * @param {T | null | undefined} collection - The object to search.
 * @param {ObjectIterateeCustom<T, boolean>} [predicate] - The predicate function, partial object, property-value pair, or property name.
 * @param {number} [fromIndex] - The index to start searching from.
 * @returns {T[keyof T] | undefined} The last element that satisfies the predicate.
 *
 * @example
 * const obj = { a: { id: 1, name: 'Alice' }, b: { id: 2 }, c: { id: 3, name: 'Bob' } };
 * findLast(obj, o => o.id > 1);
 * // => { id: 3, name: 'Bob' }
 *
 * findLast(obj, { name: 'Bob' });
 * // => { id: 3, name: 'Bob' }
 *
 * findLast(obj, 'name');
 * // => { id: 3, name: 'Bob' }
 */
declare function findLast<T extends object>(collection: T | null | undefined, predicate?: ObjectIterateeCustom<T, boolean>, fromIndex?: number): T[keyof T] | undefined;

export { findLast };
