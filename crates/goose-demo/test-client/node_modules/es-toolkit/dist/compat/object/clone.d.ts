/**
 * Creates a shallow clone of the given object.
 *
 * @template T - The type of the object.
 * @param {T} obj - The object to clone.
 * @returns {T} - A shallow clone of the given object.
 *
 * @example
 * // Clone a primitive objs
 * const num = 29;
 * const clonedNum = clone(num);
 * console.log(clonedNum); // 29
 * console.log(clonedNum === num); // true
 *
 * @example
 * // Clone an array
 * const arr = [1, 2, 3];
 * const clonedArr = clone(arr);
 * console.log(clonedArr); // [1, 2, 3]
 * console.log(clonedArr === arr); // false
 *
 * @example
 * // Clone an object
 * const obj = { a: 1, b: 'es-toolkit', c: [1, 2, 3] };
 * const clonedObj = clone(obj);
 * console.log(clonedObj); // { a: 1, b: 'es-toolkit', c: [1, 2, 3] }
 * console.log(clonedObj === obj); // false
 */
declare function clone<T>(obj: T): T;

export { clone };
