/**
 * Checks if a value is a TypedArray.
 * @param {any} x The value to check.
 * @returns {boolean} Returns true if `x` is a TypedArray, false otherwise.
 *
 * @example
 * const arr = new Uint8Array([1, 2, 3]);
 * isTypedArray(arr); // true
 *
 * const regularArray = [1, 2, 3];
 * isTypedArray(regularArray); // false
 *
 * const buffer = new ArrayBuffer(16);
 * isTypedArray(buffer); // false
 */
declare function isTypedArray(x: any): boolean;

export { isTypedArray };
