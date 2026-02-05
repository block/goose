/**
 * Creates an array of the own enumerable property values of `object`.
 *
 * @template T
 * @param {Record<string, T> | Record<number, T> | ArrayLike<T> | null | undefined} object - The object to query.
 * @returns {T[]} Returns an array of property values.
 *
 * @example
 * const obj = { a: 1, b: 2, c: 3 };
 * values(obj); // => [1, 2, 3]
 */
declare function values<T>(object: Record<string, T> | Record<number, T> | ArrayLike<T> | null | undefined): T[];
/**
 * Creates an array of the own enumerable property values of `object`.
 *
 * @template T
 * @param {T | null | undefined} object - The object to query.
 * @returns {Array<T[keyof T]>} Returns an array of property values.
 *
 * @example
 * const obj = { a: 1, b: 2, c: 3 };
 * values(obj); // => [1, 2, 3]
 */
declare function values<T extends object>(object: T | null | undefined): Array<T[keyof T]>;
/**
 * Creates an array of the own enumerable property values of `object`.
 *
 * @param {any} object - The object to query.
 * @returns {any[]} Returns an array of property values.
 *
 * @example
 * const obj = { a: 1, b: 2, c: 3 };
 * values(obj); // => [1, 2, 3]
 */
declare function values(object: any): any[];

export { values };
