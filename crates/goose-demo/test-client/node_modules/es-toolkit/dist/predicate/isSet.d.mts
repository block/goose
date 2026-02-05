/**
 * Checks if a given value is `Set`.
 *
 * This function can also serve as a type predicate in TypeScript, narrowing the type of the argument to `Set`.
 *
 * @param {unknown} value The value to check if it is a `Set`.
 * @returns {value is Set<any>} Returns `true` if `value` is a `Set`, else `false`.
 *
 * @example
 * const value1 = new Set();
 * const value2 = new Map();
 * const value3 = new WeakSet();
 *
 * console.log(isSet(value1)); // true
 * console.log(isSet(value2)); // false
 * console.log(isSet(value3)); // false
 */
declare function isSet(value: unknown): value is Set<any>;

export { isSet };
