/**
 * Checks if a given value is null or undefined.
 *
 * This function tests whether the provided value is either `null` or `undefined`.
 * It returns `true` if the value is `null` or `undefined`, and `false` otherwise.
 *
 * This function can also serve as a type predicate in TypeScript, narrowing the type of the argument to `null` or `undefined`.
 *
 * @param {any} x - The value to test for null or undefined.
 * @returns {x is null | undefined} `true` if the value is null or undefined, `false` otherwise.
 *
 * @example
 * const value1 = null;
 * const value2 = undefined;
 * const value3 = 42;
 * const result1 = isNil(value1); // true
 * const result2 = isNil(value2); // true
 * const result3 = isNil(value3); // false
 */
declare function isNil(x: any): x is null | undefined;

export { isNil };
