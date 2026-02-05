/**
 * Combines two arrays, one of property names and one of corresponding values, into a single object.
 *
 * @template T - The type of values in the values array
 * @param {ArrayLike<PropertyKey>} props - An array of property names
 * @param {ArrayLike<T>} values - An array of values corresponding to the property names
 * @returns {Record<string, T>} A new object composed of the given property names and values
 *
 * @example
 * const props = ['a', 'b', 'c'];
 * const values = [1, 2, 3];
 * zipObject(props, values);
 * // => { a: 1, b: 2, c: 3 }
 */
declare function zipObject<T>(props: ArrayLike<PropertyKey>, values: ArrayLike<T>): Record<string, T>;
/**
 * Creates an object from an array of property names, with undefined values.
 *
 * @param {ArrayLike<PropertyKey>} [props] - An array of property names
 * @returns {Record<string, undefined>} A new object with the given property names and undefined values
 *
 * @example
 * const props = ['a', 'b', 'c'];
 * zipObject(props);
 * // => { a: undefined, b: undefined, c: undefined }
 */
declare function zipObject(props?: ArrayLike<PropertyKey>): Record<string, undefined>;

export { zipObject };
