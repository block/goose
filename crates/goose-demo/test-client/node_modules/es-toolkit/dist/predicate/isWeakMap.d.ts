/**
 * Checks if the given value is a `WeakMap`.
 *
 * This function tests whether the provided value is an instance of `WeakMap`.
 * It returns `true` if the value is a `WeakMap`, and `false` otherwise.
 *
 * This function can also serve as a type predicate in TypeScript, narrowing the type of the argument to `WeakMap`.
 *
 * @param {unknown} value - The value to test if it is a `WeakMap`.
 * @returns {value is WeakMap<WeakKey, any>} true if the value is a `WeakMap`, false otherwise.
 *
 * @example
 * const value1 = new WeakMap();
 * const value2 = new Map();
 * const value3 = new Set();
 *
 * console.log(isWeakMap(value1)); // true
 * console.log(isWeakMap(value2)); // false
 * console.log(isWeakMap(value3)); // false
 */
declare function isWeakMap(value: unknown): value is WeakMap<WeakKey, any>;

export { isWeakMap };
