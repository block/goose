/**
 * Computes number rounded up to precision.
 *
 * @param {number | string} number The number to round up.
 * @param {number | string} precision The precision to round up to.
 * @returns {number} Returns the rounded up number.
 *
 * @example
 * ceil(4.006); // => 5
 * ceil(6.004, 2); // => 6.01
 * ceil(6040, -2); // => 6100
 */
declare function ceil(number: number, precision?: number): number;

export { ceil };
