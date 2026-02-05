/**
 * Checks if the given value is a `WeakSet`.
 *
 * This function tests whether the provided value is an instance of `WeakSet`.
 * It returns `true` if the value is a `WeakSet`, and `false` otherwise.
 *
 * This function can also serve as a type predicate in TypeScript, narrowing the type of the argument to `WeakSet`.
 *
 * @param {unknown} value - The value to test if it is a `WeakSet`.
 * @returns {value is WeakSet<WeakKey>} true if the value is a `WeakSet`, false otherwise.
 *
 * @example
 * const value1 = new WeakSet();
 * const value2 = new Map();
 * const value3 = new Set();
 *
 * console.log(isWeakSet(value1)); // true
 * console.log(isWeakSet(value2)); // false
 * console.log(isWeakSet(value3)); // false
 */
declare function isWeakSet(value?: any): value is WeakSet<object>;

export { isWeakSet };
