/**
 * Generate a random number between 0 and 1.
 * @param {boolean} [floating] - Whether to return a floating point number. Defaults to true.
 * @returns {number} A random number between 0 and 1.
 * @example
 * random(); // Returns a random number between 0 and 1
 * random(true); // Returns a random floating point number between 0 and 1
 * random(false); // Returns a random integer between 0 and 1
 */
declare function random(floating?: boolean): number;
/**
 * Generate a random number between 0 and max.
 * @param {number} max - The upper bound (exclusive).
 * @param {boolean} [floating] - Whether to return a floating point number. Defaults to true.
 * @returns {number} A random number between 0 and max.
 * @example
 * random(5); // Returns a random number between 0 and 5
 * random(10, true); // Returns a random floating point number between 0 and 10
 * random(3, false); // Returns a random integer between 0 and 3
 */
declare function random(max: number, floating?: boolean): number;
/**
 * Generate a random number between min and max.
 * @param {number} min - The lower bound (inclusive).
 * @param {number} max - The upper bound (exclusive).
 * @param {boolean} [floating] - Whether to return a floating point number. Defaults to true.
 * @returns {number} A random number between min and max.
 * @example
 * random(1, 5); // Returns a random number between 1 and 5
 * random(0, 10, true); // Returns a random floating point number between 0 and 10
 * random(1, 6, false); // Returns a random integer between 1 and 6
 */
declare function random(min: number, max: number, floating?: boolean): number;
/**
 * Generate a random number between 0 and min, using guard object for special cases.
 * @param {number} min - The upper bound (exclusive).
 * @param {string | number} index - The index or key to check in the guard object.
 * @param {object} guard - The guard object to validate the parameters.
 * @returns {number} A random number between 0 and min.
 * @example
 * const guard = { 5: 5 };
 * random(5, 5, guard); // Returns a random number between 0 and 5
 */
declare function random(min: number, index: string | number, guard: object): number;

export { random };
