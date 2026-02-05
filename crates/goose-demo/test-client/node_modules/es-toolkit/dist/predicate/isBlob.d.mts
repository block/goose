/**
 * Checks if the given value is a Blob.
 *
 * This function tests whether the provided value is an instance of `Blob`.
 * It returns `true` if the value is an instance of `Blob`, and `false` otherwise.
 *
 * @param {unknown} x - The value to test if it is a Blob.
 * @returns {x is Blob} True if the value is a Blob, false otherwise.
 *
 * @example
 * const value1 = new Blob();
 * const value2 = {};
 *
 * console.log(isBlob(value1)); // true
 * console.log(isBlob(value2)); // false
 */
declare function isBlob(x: unknown): x is Blob;

export { isBlob };
