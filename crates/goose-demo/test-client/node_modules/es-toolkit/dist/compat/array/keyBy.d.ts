import { ValueIterateeCustom } from '../_internal/ValueIterateeCustom.js';

/**
 * Creates an object composed of keys generated from the results of running each element of collection thru iteratee.
 *
 * @template T
 * @param {ArrayLike<T> | null | undefined} collection - The collection to iterate over.
 * @param {ValueIterateeCustom<T, PropertyKey>} [iteratee] - The iteratee to transform keys.
 * @returns {Record<string, T>} Returns the composed aggregate object.
 *
 * @example
 * const array = [
 *   { dir: 'left', code: 97 },
 *   { dir: 'right', code: 100 }
 * ];
 *
 * keyBy(array, o => String.fromCharCode(o.code));
 * // => { a: { dir: 'left', code: 97 }, d: { dir: 'right', code: 100 } }
 *
 * keyBy(array, 'dir');
 * // => { left: { dir: 'left', code: 97 }, right: { dir: 'right', code: 100 } }
 */
declare function keyBy<T>(collection: ArrayLike<T> | null | undefined, iteratee?: ValueIterateeCustom<T, PropertyKey>): Record<string, T>;
/**
 * Creates an object composed of keys generated from the results of running each element of collection thru iteratee.
 *
 * @template T
 * @param {T | null | undefined} collection - The object to iterate over.
 * @param {ValueIterateeCustom<T[keyof T], PropertyKey>} [iteratee] - The iteratee to transform keys.
 * @returns {Record<string, T[keyof T]>} Returns the composed aggregate object.
 *
 * @example
 * const obj = { a: { dir: 'left', code: 97 }, b: { dir: 'right', code: 100 } };
 * keyBy(obj, o => String.fromCharCode(o.code));
 * // => { a: { dir: 'left', code: 97 }, d: { dir: 'right', code: 100 } }
 */
declare function keyBy<T extends object>(collection: T | null | undefined, iteratee?: ValueIterateeCustom<T[keyof T], PropertyKey>): Record<string, T[keyof T]>;

export { keyBy };
