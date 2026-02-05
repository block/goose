/**
 * Generates a random integer between 0 (inclusive) and the given maximum (exclusive).
 *
 * @param {number} maximum - The upper bound (exclusive).
 * @returns {number} A random integer between 0 (inclusive) and maximum (exclusive).
 * @throws {Error} Throws an error if `maximum` is not greater than `0`.
 *
 * @example
 * const result = randomInt(5); // result will be a random integer between 0 (inclusive) and 5 (exclusive)
 */
declare function randomInt(maximum: number): number;
/**
 * Generates a random integer between minimum (inclusive) and maximum (exclusive).
 *
 * @param {number} minimum - The lower bound (inclusive).
 * @param {number} maximum - The upper bound (exclusive).
 * @returns {number} A random integer between minimum (inclusive) and maximum (exclusive).
 * @throws {Error} Throws an error if `maximum` is not greater than `minimum`.
 *
 * @example
 * const result = randomInt(0, 5); // result will be a random integer between 0 (inclusive) and 5 (exclusive)
 * const result2 = randomInt(5, 0); // This will throw an error
 */
declare function randomInt(minimum: number, maximum: number): number;

export { randomInt };
