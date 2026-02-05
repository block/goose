import { ListIterateeCustom } from '../_internal/ListIterateeCustom.js';
import { ListIteratorTypeGuard } from '../_internal/ListIteratorTypeGuard.js';
import { ObjectIterateeCustom } from '../_internal/ObjectIteratee.js';
import { ObjectIteratorTypeGuard } from '../_internal/ObjectIterator.js';

/**
 * Finds the first element in an array-like object that matches a type guard predicate.
 *
 * @param collection - The array-like object to search
 * @param predicate - The type guard function to test each element
 * @param fromIndex - The index to start searching from
 * @returns The first element that matches the type guard, or undefined if none found
 *
 * @example
 * find([1, '2', 3], (x): x is number => typeof x === 'number')
 * // => 1
 */
declare function find<T, U extends T>(collection: ArrayLike<T> | null | undefined, predicate: ListIteratorTypeGuard<T, U>, fromIndex?: number): U | undefined;
/**
 * Finds the first element in an array-like object that matches a predicate.
 *
 * @param collection - The array-like object to search
 * @param predicate - The function or shorthand to test each element
 * @param fromIndex - The index to start searching from
 * @returns The first matching element, or undefined if none found
 *
 * @example
 * find([1, 2, 3], x => x > 2)
 * // => 3
 *
 * find([{ a: 1 }, { a: 2 }], { a: 2 })
 * // => { a: 2 }
 */
declare function find<T>(collection: ArrayLike<T> | null | undefined, predicate?: ListIterateeCustom<T, boolean>, fromIndex?: number): T | undefined;
/**
 * Finds the first value in an object that matches a type guard predicate.
 *
 * @param collection - The object to search
 * @param predicate - The type guard function to test each value
 * @param fromIndex - The index to start searching from
 * @returns The first value that matches the type guard, or undefined if none found
 *
 * @example
 * find({ a: 1, b: '2', c: 3 }, (x): x is number => typeof x === 'number')
 * // => 1
 */
declare function find<T extends object, U extends T[keyof T]>(collection: T | null | undefined, predicate: ObjectIteratorTypeGuard<T, U>, fromIndex?: number): U | undefined;
/**
 * Finds the first value in an object that matches a predicate.
 *
 * @param collection - The object to search
 * @param predicate - The function or shorthand to test each value
 * @param fromIndex - The index to start searching from
 * @returns The first matching value, or undefined if none found
 *
 * @example
 * find({ a: 1, b: 2 }, x => x > 1)
 * // => 2
 *
 * find({ a: { x: 1 }, b: { x: 2 } }, { x: 2 })
 * // => { x: 2 }
 */
declare function find<T extends object>(collection: T | null | undefined, predicate?: ObjectIterateeCustom<T, boolean>, fromIndex?: number): T[keyof T] | undefined;

export { find };
