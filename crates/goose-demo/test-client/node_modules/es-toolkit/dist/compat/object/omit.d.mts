import { Many } from '../_internal/Many.mjs';

/**
 * Creates a new object with specified keys omitted.
 *
 * @template T - The type of object.
 * @template K - The type of keys to omit.
 * @param {T | null | undefined} object - The object to omit keys from.
 * @param {...K} paths - The keys to be omitted from the object.
 * @returns {Pick<T, Exclude<keyof T, K[number]>>} A new object with the specified keys omitted.
 *
 * @example
 * omit({ a: 1, b: 2, c: 3 }, 'a', 'c');
 * // => { b: 2 }
 */
declare function omit<T extends object, K extends PropertyKey[]>(object: T | null | undefined, ...paths: K): Pick<T, Exclude<keyof T, K[number]>>;
/**
 * Creates a new object with specified keys omitted.
 *
 * @template T - The type of object.
 * @template K - The type of keys to omit.
 * @param {T | null | undefined} object - The object to omit keys from.
 * @param {...Array<Many<K>>} paths - The keys to be omitted from the object.
 * @returns {Omit<T, K>} A new object with the specified keys omitted.
 *
 * @example
 * omit({ a: 1, b: 2, c: 3 }, 'a', ['b', 'c']);
 * // => {}
 */
declare function omit<T extends object, K extends keyof T>(object: T | null | undefined, ...paths: Array<Many<K>>): Omit<T, K>;
/**
 * Creates a new object with specified keys omitted.
 *
 * @template T - The type of object.
 * @param {T | null | undefined} object - The object to omit keys from.
 * @param {...Array<Many<PropertyKey>>} paths - The keys to be omitted from the object.
 * @returns {Partial<T>} A new object with the specified keys omitted.
 *
 * @example
 * omit({ a: 1, b: 2, c: 3 }, 'a', 'b');
 * // => { c: 3 }
 */
declare function omit<T extends object>(object: T | null | undefined, ...paths: Array<Many<PropertyKey>>): Partial<T>;

export { omit };
