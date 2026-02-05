/**
 * Checks if `value` is `null`.
 *
 * @param {any} value - The value to check.
 * @returns {value is null} Returns `true` if `value` is `null`, else `false`.
 *
 * @example
 * isNull(null); // true
 * isNull(undefined); // false
 * isNull(0); // false
 */
declare function isNull(value: any): value is null;

export { isNull };
