/**
 * Checks if the given value is a number.
 *
 * This function tests whether the provided value is strictly a `number`.
 * It returns `true` if the value is a `number`, and `false` otherwise.
 *
 * This function can also serve as a type predicate in TypeScript, narrowing the type of the argument to `number`.
 *
 * @param {unknown} x - The value to test if it is a number.
 * @returns {x is number} True if the value is a number, false otherwise.
 *
 * @example
 *
 * const value1 = 123;
 * const value2 = 'abc';
 * const value3 = true;
 *
 * console.log(isNumber(value1)); // true
 * console.log(isNumber(value2)); // false
 * console.log(isNumber(value3)); // false
 *
 */
declare function isNumber(x: unknown): x is number;

export { isNumber };
