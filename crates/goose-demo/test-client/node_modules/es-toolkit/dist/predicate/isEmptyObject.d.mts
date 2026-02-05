/**
 * Checks if a value is an empty plain object.
 *
 * @param {unknown} value - The value to check.
 * @returns {value is Record<PropertyKey, never>} - True if the value is an empty plain object, otherwise false.
 *
 * @example
 * isEmptyObject({}); // true
 * isEmptyObject({ a: 1 }); // false
 * isEmptyObject([]); // false
 * isEmptyObject(null); // false
 */
declare function isEmptyObject(value: unknown): value is Record<PropertyKey, never>;

export { isEmptyObject };
