/**
 * Checks if the given value is a Buffer instance.
 *
 * This function tests whether the provided value is an instance of Buffer.
 * It returns `true` if the value is a Buffer, and `false` otherwise.
 *
 * This function can also serve as a type predicate in TypeScript, narrowing the type of the argument to `Buffer`.
 *
 * @param {any} x - The value to check if it is a Buffer.
 * @returns {boolean} Returns `true` if `x` is a Buffer, else `false`.
 *
 * @example
 * const buffer = Buffer.from("test");
 * console.log(isBuffer(buffer)); // true
 *
 * const notBuffer = "not a buffer";
 * console.log(isBuffer(notBuffer)); // false
 */
declare function isBuffer(x?: any): boolean;

export { isBuffer };
