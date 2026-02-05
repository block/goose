/**
 * Recursively merges own and inherited enumerable string keyed properties of source objects into the destination object.
 *
 * @template T
 * @template U
 * @param {T} object - The destination object.
 * @param {U} source - The source object.
 * @returns {T & U} - Returns `object`.
 *
 * @example
 * const object = { a: [{ b: 2 }, { d: 4 }] };
 * const other = { a: [{ c: 3 }, { e: 5 }] };
 * merge(object, other);
 * // => { a: [{ b: 2, c: 3 }, { d: 4, e: 5 }] }
 */
declare function merge<T, U>(object: T, source: U): T & U;
/**
 * Recursively merges own and inherited enumerable string keyed properties of source objects into the destination object.
 *
 * @template T
 * @template U
 * @template V
 * @param {T} object - The destination object.
 * @param {U} source1 - The first source object.
 * @param {V} source2 - The second source object.
 * @returns {T & U & V} - Returns `object`.
 *
 * @example
 * merge({ a: 1 }, { b: 2 }, { c: 3 });
 * // => { a: 1, b: 2, c: 3 }
 */
declare function merge<T, U, V>(object: T, source1: U, source2: V): T & U & V;
/**
 * Recursively merges own and inherited enumerable string keyed properties of source objects into the destination object.
 *
 * @template T
 * @template U
 * @template V
 * @template W
 * @param {T} object - The destination object.
 * @param {U} source1 - The first source object.
 * @param {V} source2 - The second source object.
 * @param {W} source3 - The third source object.
 * @returns {T & U & V & W} - Returns `object`.
 *
 * @example
 * merge({ a: 1 }, { b: 2 }, { c: 3 }, { d: 4 });
 * // => { a: 1, b: 2, c: 3, d: 4 }
 */
declare function merge<T, U, V, W>(object: T, source1: U, source2: V, source3: W): T & U & V & W;
/**
 * Recursively merges own and inherited enumerable string keyed properties of source objects into the destination object.
 *
 * @template T
 * @template U
 * @template V
 * @template W
 * @template X
 * @param {T} object - The destination object.
 * @param {U} source1 - The first source object.
 * @param {V} source2 - The second source object.
 * @param {W} source3 - The third source object.
 * @param {X} source4 - The fourth source object.
 * @returns {T & U & V & W & X} - Returns `object`.
 *
 * @example
 * merge({ a: 1 }, { b: 2 }, { c: 3 }, { d: 4 }, { e: 5 });
 * // => { a: 1, b: 2, c: 3, d: 4, e: 5 }
 */
declare function merge<T, U, V, W, X>(object: T, source1: U, source2: V, source3: W, source4: X): T & U & V & W & X;
/**
 * Recursively merges own and inherited enumerable string keyed properties of source objects into the destination object.
 *
 * @param {any} object - The destination object.
 * @param {...any[]} otherArgs - The source objects.
 * @returns {any} - Returns `object`.
 *
 * @example
 * merge({ a: 1 }, { b: 2 }, { c: 3 });
 * // => { a: 1, b: 2, c: 3 }
 */
declare function merge(object: any, ...otherArgs: any[]): any;

export { merge };
