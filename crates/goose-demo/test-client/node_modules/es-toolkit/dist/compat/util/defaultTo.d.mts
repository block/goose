/**
 * Returns the default value for `null`, `undefined`, and `NaN`.
 *
 * @template T - The type of the value parameter
 * @param {T | null | undefined} value - The value to check.
 * @param {T} defaultValue - The default value to return if the first value is null, undefined, or NaN.
 * @returns {T} Returns either the first value or the default value.
 */
declare function defaultTo<T>(value: T | null | undefined, defaultValue: T): T;
/**
 * Returns the default value for `null`, `undefined`, and `NaN`.
 *
 * @template T - The type of the value parameter
 * @template D - The type of the defaultValue parameter
 * @param {T | null | undefined} value - The value to check.
 * @param {D} defaultValue - The default value to return if the first value is null, undefined, or NaN.
 * @returns {T | D} Returns either the first value or the default value.
 */
declare function defaultTo<T, D>(value: T | null | undefined, defaultValue: D): T | D;

export { defaultTo };
