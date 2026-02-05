/**
 * Clamps a number within the specified bounds.
 *
 * @param {number} number The number to clamp
 * @param {number} lower The lower bound
 * @param {number} upper The upper bound
 * @returns {number} Returns the clamped number
 * @example
 * clamp(3, 2, 4) // => 3
 * clamp(0, 5, 10) // => 5
 * clamp(15, 5, 10) // => 10
 */
declare function clamp(number: number, lower: number, upper: number): number;
/**
 * Clamps a number to an upper bound.
 *
 * @param {number} number The number to clamp
 * @param {number} upper The upper bound
 * @returns {number} Returns the clamped number
 * @example
 * clamp(5, 3) // => 3
 * clamp(2, 3) // => 2
 */
declare function clamp(number: number, upper: number): number;

export { clamp };
