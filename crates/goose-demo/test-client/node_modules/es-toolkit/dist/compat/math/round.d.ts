/**
 * Computes number rounded to precision.
 *
 * @param {number} number  The number to round.
 * @param {number} precision The precision to round to.
 * @returns {number} Returns the rounded number.
 *
 * @example
 * round(4.006); // => 4
 * round(4.006, 2); // => 4.01
 * round(4060, -2); // => 4100
 */
declare function round(number: number, precision?: number): number;

export { round };
