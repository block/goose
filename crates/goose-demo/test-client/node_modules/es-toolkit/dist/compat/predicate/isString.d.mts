/**
 * Checks if a given value is string.
 *
 * This function can also serve as a type predicate in TypeScript, narrowing the type of the argument to `string`.
 *
 * @param {unknown} value The value to check if it is string.
 * @returns {value is string} Returns `true` if `value` is a string, else `false`.
 *
 * @example
 * const value1 = 'abc';
 * const value2 = 123;
 * const value3 = true;
 *
 * console.log(isString(value1)); // true
 * console.log(isString(value2)); // false
 * console.log(isString(value3)); // false
 */
declare function isString(value?: any): value is string;

export { isString };
