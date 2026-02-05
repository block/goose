/**
 * Checks if a given value is `Promise`.
 *
 * This function can also serve as a type predicate in TypeScript, narrowing the type of the argument to `Promise`.
 *
 * @param {unknown} value The value to check if it is a `Promise`.
 * @returns {value is Promise<any>} Returns `true` if `value` is a `Promise`, else `false`.
 *
 * @example
 * const value1 = new Promise((resolve) => resolve());
 * const value2 = {};
 * const value3 = 123;
 *
 * console.log(isPromise(value1)); // true
 * console.log(isPromise(value2)); // false
 * console.log(isPromise(value3)); // false
 */
declare function isPromise(value: unknown): value is Promise<any>;

export { isPromise };
