import { ValueIteratee } from '../_internal/ValueIteratee.mjs';
import { ValueIteratorTypeGuard } from '../_internal/ValueIteratorTypeGuard.mjs';

/**
 * Creates an array of elements split into two groups, the first of which contains elements
 * predicate returns truthy for, while the second of which contains elements predicate returns falsey for.
 * The predicate is invoked with one argument: (value).
 *
 * @template T, U
 * @param {ArrayLike<T> | null | undefined} collection - The collection to iterate over.
 * @param {(value: T) => value is U} callback - The function invoked per iteration.
 * @returns {[U[], Array<Exclude<T, U>>]} Returns the array of grouped elements.
 *
 * @example
 * partition([1, 2, 3, 4], n => n % 2 === 0);
 * // => [[2, 4], [1, 3]]
 */
declare function partition<T, U extends T>(collection: ArrayLike<T> | null | undefined, callback: ValueIteratorTypeGuard<T, U>): [U[], Array<Exclude<T, U>>];
/**
 * Creates an array of elements split into two groups, the first of which contains elements
 * predicate returns truthy for, while the second of which contains elements predicate returns falsey for.
 * The predicate is invoked with one argument: (value).
 *
 * @template T
 * @param {ArrayLike<T> | null | undefined} collection - The collection to iterate over.
 * @param {((value: T) => unknown) | PropertyKey | [PropertyKey, any] | Partial<T>} callback - The function invoked per iteration.
 * @returns {[T[], T[]]} Returns the array of grouped elements.
 *
 * @example
 * partition([1, 2, 3, 4], n => n % 2 === 0);
 * // => [[2, 4], [1, 3]]
 */
declare function partition<T>(collection: ArrayLike<T> | null | undefined, callback: ValueIteratee<T>): [T[], T[]];
/**
 * Creates an array of elements split into two groups, the first of which contains elements
 * predicate returns truthy for, while the second of which contains elements predicate returns falsey for.
 * The predicate is invoked with one argument: (value).
 *
 * @template T
 * @param {T | null | undefined} collection - The collection to iterate over.
 * @param {((value: T[keyof T]) => unknown) | PropertyKey | [PropertyKey, any] | Partial<T[keyof T]>} callback - The function invoked per iteration.
 * @returns {[Array<T[keyof T]>, Array<T[keyof T]>]} Returns the array of grouped elements.
 *
 * @example
 * partition({ a: 1, b: 2, c: 3 }, n => n % 2 === 0);
 * // => [[2], [1, 3]]
 */
declare function partition<T extends object>(collection: T | null | undefined, callback: ValueIteratee<T[keyof T]>): [Array<T[keyof T]>, Array<T[keyof T]>];

export { partition };
