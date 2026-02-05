/**
 * Checks if a value is a TypedArray.
 * @param {unknown} x The value to check.
 * @returns {x is
 *     Uint8Array
 *   | Uint8ClampedArray
 *   | Uint16Array
 *   | Uint32Array
 *   | BigUint64Array
 *   | Int8Array
 *   | Int16Array
 *   | Int32Array
 *   | BigInt64Array
 *   | Float32Array
 *   | Float64Array} Returns true if `x` is a TypedArray, false otherwise.
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
declare function isTypedArray(x: unknown): x is Uint8Array | Uint8ClampedArray | Uint16Array | Uint32Array | BigUint64Array | Int8Array | Int16Array | Int32Array | BigInt64Array | Float32Array | Float64Array;

export { isTypedArray };
