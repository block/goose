/**
 * Checks if the given value is null.
 *
 * This function tests whether the provided value is strictly equal to `null`.
 * It returns `true` if the value is `null`, and `false` otherwise.
 *
 * This function can also serve as a type predicate in TypeScript, narrowing the type of the argument to `null`.
 *
 * @param {unknown} x - The value to test if it is null.
 * @returns {x is null} True if the value is null, false otherwise.
 *
 * @example
 * const value1 = null;
 * const value2 = undefined;
 * const value3 = 42;
 *
 * console.log(isNull(value1)); // true
 * console.log(isNull(value2)); // false
 * console.log(isNull(value3)); // false
 */
declare function isNull(x: unknown): x is null;

export { isNull };
