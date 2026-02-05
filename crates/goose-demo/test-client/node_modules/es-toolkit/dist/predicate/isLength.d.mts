/**
 * Checks if a given value is a valid length.
 *
 * A valid length is of type `number`, is a non-negative integer, and is less than or equal to
 * JavaScript's maximum safe integer (`Number.MAX_SAFE_INTEGER`).
 * It returns `true` if the value is a valid length, and `false` otherwise.
 *
 * This function can also serve as a type predicate in TypeScript, narrowing the type of the
 * argument to a valid length (`number`).
 *
 * @param {any} value The value to check.
 * @returns {boolean} Returns `true` if `value` is a valid length, else `false`.
 *
 * @example
 * isLength(0); // true
 * isLength(42); // true
 * isLength(-1); // false
 * isLength(1.5); // false
 * isLength(Number.MAX_SAFE_INTEGER); // true
 * isLength(Number.MAX_SAFE_INTEGER + 1); // false
 */
declare function isLength(value?: any): boolean;

export { isLength };
