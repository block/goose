/**
 * Generate a random number within the given range.
 *
 * If only one argument is provided, a number between `0` and the given number is returned.
 *
 * @param {number} maximum - The upper bound (exclusive).
 * @returns {number} A random number between 0 (inclusive) and maximum (exclusive). The number can be an integer or a decimal.
 * @throws {Error} Throws an error if `maximum` is not greater than `0`.
 *
 * @example
 * const result1 = random(5); // Returns a random number between 0 and 5.
 * const result2 = random(0); // If the `maximum` is less than or equal to 0, an error is thrown.
 */
declare function random(maximum: number): number;
/**
 * Generate a random number within the given range.
 *
 * @param {number} minimum - The lower bound (inclusive).
 * @param {number} maximum - The upper bound (exclusive).
 * @returns {number} A random number between minimum (inclusive) and maximum (exclusive). The number can be an integer or a decimal.
 * @throws {Error} Throws an error if `maximum` is not greater than `minimum`.
 *
 * @example
 * const result1 = random(0, 5); // Returns a random number between 0 and 5.
 * const result2 = random(5, 0); // If the minimum is greater than the maximum, an error is thrown.
 * const result3 = random(5, 5); // If the minimum is equal to the maximum, an error is thrown.
 */
declare function random(minimum: number, maximum: number): number;

export { random };
