/**
 * Assigns own and inherited properties from one source object to a target object.
 *
 * @template T - The type of the target object.
 * @template U - The type of the source object.
 * @param {T} object - The target object to which properties will be assigned.
 * @param {U} source - The source object whose properties will be assigned to the target object.
 * @returns {T & U} The updated target object with properties from the source object assigned.
 *
 * @example
 * const target = { a: 1, b: 2 };
 * const source = { b: 3, c: 4 };
 * const result = assignIn(target, source);
 * // => { a: 1, b: 3, c: 4 }
 */
declare function assignIn<T, U>(object: T, source: U): T & U;
/**
 * Assigns own and inherited properties from two source objects to a target object.
 *
 * @template T - The type of the target object.
 * @template U - The type of the first source object.
 * @template V - The type of the second source object.
 * @param {T} object - The target object to which properties will be assigned.
 * @param {U} source1 - The first source object whose properties will be assigned to the target object.
 * @param {V} source2 - The second source object whose properties will be assigned to the target object.
 * @returns {T & U & V} The updated target object with properties from the source objects assigned.
 *
 * @example
 * const target = { a: 1 };
 * const source1 = { b: 2 };
 * const source2 = { c: 3 };
 * const result = assignIn(target, source1, source2);
 * // => { a: 1, b: 2, c: 3 }
 */
declare function assignIn<T, U, V>(object: T, source1: U, source2: V): T & U & V;
/**
 * Assigns own and inherited properties from three source objects to a target object.
 *
 * @template T - The type of the target object.
 * @template U - The type of the first source object.
 * @template V - The type of the second source object.
 * @template W - The type of the third source object.
 * @param {T} object - The target object to which properties will be assigned.
 * @param {U} source1 - The first source object whose properties will be assigned to the target object.
 * @param {V} source2 - The second source object whose properties will be assigned to the target object.
 * @param {W} source3 - The third source object whose properties will be assigned to the target object.
 * @returns {T & U & V & W} The updated target object with properties from the source objects assigned.
 *
 * @example
 * const target = { a: 1 };
 * const source1 = { b: 2 };
 * const source2 = { c: 3 };
 * const source3 = { d: 4 };
 * const result = assignIn(target, source1, source2, source3);
 * // => { a: 1, b: 2, c: 3, d: 4 }
 */
declare function assignIn<T, U, V, W>(object: T, source1: U, source2: V, source3: W): T & U & V & W;
/**
 * Assigns own and inherited properties from four source objects to a target object.
 *
 * @template T - The type of the target object.
 * @template U - The type of the first source object.
 * @template V - The type of the second source object.
 * @template W - The type of the third source object.
 * @template X - The type of the fourth source object.
 * @param {T} object - The target object to which properties will be assigned.
 * @param {U} source1 - The first source object whose properties will be assigned to the target object.
 * @param {V} source2 - The second source object whose properties will be assigned to the target object.
 * @param {W} source3 - The third source object whose properties will be assigned to the target object.
 * @param {X} source4 - The fourth source object whose properties will be assigned to the target object.
 * @returns {T & U & V & W & X} The updated target object with properties from the source objects assigned.
 *
 * @example
 * const target = { a: 1 };
 * const source1 = { b: 2 };
 * const source2 = { c: 3 };
 * const source3 = { d: 4 };
 * const source4 = { e: 5 };
 * const result = assignIn(target, source1, source2, source3, source4);
 * // => { a: 1, b: 2, c: 3, d: 4, e: 5 }
 */
declare function assignIn<T, U, V, W, X>(object: T, source1: U, source2: V, source3: W, source4: X): T & U & V & W & X;
/**
 * Returns the target object as-is.
 *
 * @template T - The type of the target object.
 * @param {T} object - The target object.
 * @returns {T} The target object.
 *
 * @example
 * const target = { a: 1, b: 2 };
 * const result = assignIn(target);
 * // => { a: 1, b: 2 }
 */
declare function assignIn<T>(object: T): T;
/**
 * Assigns own and inherited properties from multiple source objects to a target object.
 *
 * @template R - The type of the result.
 * @param {any} object - The target object to which properties will be assigned.
 * @param {...any[]} otherArgs - The source objects whose properties will be assigned to the target object.
 * @returns {R} The updated target object with properties from the source objects assigned.
 *
 * @example
 * const target = { a: 1 };
 * const result = assignIn(target, { b: 2 }, { c: 3 }, { d: 4 });
 * // => { a: 1, b: 2, c: 3, d: 4 }
 */
declare function assignIn<R>(object: any, ...otherArgs: any[]): R;

export { assignIn };
