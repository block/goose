/**
 * Checks if a given value is `Map`.
 *
 * This function can also serve as a type predicate in TypeScript, narrowing the type of the argument to `Map`.
 *
 * @param {unknown} value The value to check if it is a `Map`.
 * @returns {value is Map<any, any>} Returns `true` if `value` is a `Map`, else `false`.
 *
 * @example
 * const value1 = new Map();
 * const value2 = new Set();
 * const value3 = new WeakMap();
 *
 * console.log(isMap(value1)); // true
 * console.log(isMap(value2)); // false
 * console.log(isMap(value3)); // false
 */
declare function isMap(value?: any): value is Map<any, any>;

export { isMap };
