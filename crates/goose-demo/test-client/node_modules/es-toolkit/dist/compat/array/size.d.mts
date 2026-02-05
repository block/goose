/**
 * Returns the length of an array, string, or object.
 *
 * This function takes an array, string, or object and returns its length.
 * For arrays and strings, it returns the number of elements or characters, respectively.
 * For objects, it returns the number of enumerable properties.
 *
 * @template T - The type of the input value.
 * @param {T[] | object | string | Map<unknown, T> | Set<T> | null | undefined } target - The value whose size is to be determined. It can be an array, string, or object.
 * @returns {number} The size of the input value.
 *
 * @example
 * const arr = [1, 2, 3];
 * const arrSize = size(arr);
 * // arrSize will be 3
 *
 * const str = 'hello';
 * const strSize = size(str);
 * // strSize will be 5
 *
 * const obj = { a: 1, b: 2, c: 3 };
 * const objSize = size(obj);
 * // objSize will be 3
 *
 * const emptyArr = [];
 * const emptyArrSize = size(emptyArr);
 * // emptyArrSize will be 0
 *
 * const emptyStr = '';
 * const emptyStrSize = size(emptyStr);
 * // emptyStrSize will be 0
 *
 * const emptyObj = {};
 * const emptyObjSize = size(emptyObj);
 * // emptyObjSize will be 0
 */
declare function size(collection: object | string | null | undefined): number;

export { size };
