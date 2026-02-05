import { ValueIteratee } from '../_internal/ValueIteratee.mjs';

/**
 * Creates an object composed of keys generated from the results of running each element of collection through iteratee.
 * The order of grouped values is determined by the order they occur in collection.
 *
 * @template T - The type of elements in the array-like collection
 * @param {ArrayLike<T> | null | undefined} collection - The collection to iterate over
 * @param {ValueIteratee<T>} [iteratee=identity] - The iteratee to transform keys
 * @returns {Record<string, T[]>} Returns the composed aggregate object
 *
 * @example
 * groupBy([6.1, 4.2, 6.3], Math.floor)
 * // => { '4': [4.2], '6': [6.1, 6.3] }
 *
 * groupBy(['one', 'two', 'three'], 'length')
 * // => { '3': ['one', 'two'], '5': ['three'] }
 */
declare function groupBy<T>(collection: ArrayLike<T> | null | undefined, iteratee?: ValueIteratee<T>): Record<string, T[]>;
/**
 * Creates an object composed of keys generated from the results of running each element of collection through iteratee.
 * The order of grouped values is determined by the order they occur in collection.
 *
 * @template T - The type of the object
 * @param {T | null | undefined} collection - The object to iterate over
 * @param {ValueIteratee<T[keyof T]>} [iteratee=identity] - The iteratee to transform keys
 * @returns {Record<string, Array<T[keyof T]>>} Returns the composed aggregate object
 *
 * @example
 * groupBy({ a: 6.1, b: 4.2, c: 6.3 }, Math.floor)
 * // => { '4': [4.2], '6': [6.1, 6.3] }
 */
declare function groupBy<T extends object>(collection: T | null | undefined, iteratee?: ValueIteratee<T[keyof T]>): Record<string, Array<T[keyof T]>>;

export { groupBy };
