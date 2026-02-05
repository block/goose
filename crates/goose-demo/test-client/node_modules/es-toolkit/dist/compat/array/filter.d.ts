import { ListIterateeCustom } from '../_internal/ListIterateeCustom.js';
import { ListIteratorTypeGuard } from '../_internal/ListIteratorTypeGuard.js';
import { ObjectIterateeCustom } from '../_internal/ObjectIteratee.js';
import { ObjectIteratorTypeGuard } from '../_internal/ObjectIterator.js';
import { StringIterator } from '../_internal/StringIterator.js';

/**
 * Filters characters in a string based on the predicate function.
 *
 * @param collection - The string to filter
 * @param predicate - The function to test each character
 * @returns An array of characters that pass the predicate test
 *
 * @example
 * filter('123', char => char === '2')
 * // => ['2']
 */
declare function filter(collection: string | null | undefined, predicate?: StringIterator<boolean>): string[];
/**
 * Filters elements in an array-like object using a type guard predicate.
 *
 * @param collection - The array-like object to filter
 * @param predicate - The type guard function to test each element
 * @returns An array of elements that are of type U
 *
 * @example
 * filter([1, '2', 3], (x): x is number => typeof x === 'number')
 * // => [1, 3]
 */
declare function filter<T, U extends T>(collection: ArrayLike<T> | null | undefined, predicate: ListIteratorTypeGuard<T, U>): U[];
/**
 * Filters elements in an array-like object based on the predicate.
 *
 * @param collection - The array-like object to filter
 * @param predicate - The function or shorthand to test each element
 * @returns An array of elements that pass the predicate test
 *
 * @example
 * filter([1, 2, 3], x => x > 1)
 * // => [2, 3]
 *
 * filter([{ a: 1 }, { a: 2 }], { a: 1 })
 * // => [{ a: 1 }]
 */
declare function filter<T>(collection: ArrayLike<T> | null | undefined, predicate?: ListIterateeCustom<T, boolean>): T[];
/**
 * Filters values in an object using a type guard predicate.
 *
 * @param collection - The object to filter
 * @param predicate - The type guard function to test each value
 * @returns An array of values that are of type U
 *
 * @example
 * filter({ a: 1, b: '2', c: 3 }, (x): x is number => typeof x === 'number')
 * // => [1, 3]
 */
declare function filter<T extends object, U extends T[keyof T]>(collection: T | null | undefined, predicate: ObjectIteratorTypeGuard<T, U>): U[];
/**
 * Filters values in an object based on the predicate.
 *
 * @param collection - The object to filter
 * @param predicate - The function or shorthand to test each value
 * @returns An array of values that pass the predicate test
 *
 * @example
 * filter({ a: 1, b: 2 }, x => x > 1)
 * // => [2]
 *
 * filter({ a: { x: 1 }, b: { x: 2 } }, { x: 1 })
 * // => [{ x: 1 }]
 */
declare function filter<T extends object>(collection: T | null | undefined, predicate?: ObjectIterateeCustom<T, boolean>): Array<T[keyof T]>;

export { filter };
