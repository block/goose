/**
 * Checks if the value is less than the maximum.
 *
 * @param {number} value The value to check.
 * @param {number} maximum The upper bound of the range (exclusive).
 * @returns {boolean} `true` if the value is less than the maximum, otherwise `false`.
 *
 * @example
 * const result = inRange(3, 5); // result will be true.
 * const result2 = inRange(5, 5); // result2 will be false.
 */
declare function inRange(value: number, maximum: number): boolean;
/**
 * Checks if the value is within the range defined by minimum (inclusive) and maximum (exclusive).
 *
 * @param {number} value The value to check.
 * @param {number} minimum The lower bound of the range (inclusive).
 * @param {number} maximum The upper bound of the range (exclusive).
 * @returns {boolean} `true` if the value is within the specified range, otherwise `false`.
 *
 * @example
 * const result = inRange(3, 2, 5); // result will be true.
 * const result2 = inRange(1, 2, 5); // result2 will be false.
 */
declare function inRange(value: number, minimum: number, maximum: number): boolean;

export { inRange };
