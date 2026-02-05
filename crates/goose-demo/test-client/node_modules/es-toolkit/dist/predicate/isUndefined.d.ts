/**
 * Checks if the given value is undefined.
 *
 * This function tests whether the provided value is strictly equal to `undefined`.
 * It returns `true` if the value is `undefined`, and `false` otherwise.
 *
 * This function can also serve as a type predicate in TypeScript, narrowing the type of the argument to `undefined`.
 *
 * @param {unknown} x - The value to test if it is undefined.
 * @returns {x is undefined} true if the value is undefined, false otherwise.
 *
 * @example
 * const value1 = undefined;
 * const value2 = null;
 * const value3 = 42;
 *
 * console.log(isUndefined(value1)); // true
 * console.log(isUndefined(value2)); // false
 * console.log(isUndefined(value3)); // false
 */
declare function isUndefined(x: any): x is undefined;

export { isUndefined };
