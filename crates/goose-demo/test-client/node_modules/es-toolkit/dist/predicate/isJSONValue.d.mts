/**
 * Checks if a given value is a valid JSON value.
 *
 * A valid JSON value can be:
 * - null
 * - a JSON object (an object with string keys and valid JSON values)
 * - a JSON array (an array of valid JSON values)
 * - a string
 * - a number
 * - a boolean
 *
 * @param {unknown} value - The value to check.
 * @returns {boolean} - True if the value is a valid JSON value, otherwise false.
 *
 * @example
 * console.log(isJSONValue(null)); // true
 * console.log(isJSONValue({ key: "value" })); // true
 * console.log(isJSONValue([1, 2, 3])); // true
 * console.log(isJSONValue("Hello")); // true
 * console.log(isJSONValue(42)); // true
 * console.log(isJSONValue(true)); // true
 * console.log(isJSONValue(undefined)); // false
 * console.log(isJSONValue(() => {})); // false
 */
declare function isJSONValue(value: unknown): value is Record<string, any> | any[] | string | number | boolean | null;
/**
 * Checks if a given value is a valid JSON array.
 *
 * A valid JSON array is defined as an array where all items are valid JSON values.
 *
 * @param {unknown} value - The value to check.
 * @returns {value is any[]} - True if the value is a valid JSON array, otherwise false.
 *
 * @example
 * console.log(isJSONArray([1, 2, 3])); // true
 * console.log(isJSONArray(["string", null, true])); // true
 * console.log(isJSONArray([1, 2, () => {}])); // false
 * console.log(isJSONArray("not an array")); // false
 */
declare function isJSONArray(value: unknown): value is any[];
/**
 * Checks if a value is a JSON object.
 *
 * A valid JSON object is defined as an object with string keys and valid JSON values.
 *
 * @param {unknown} obj The value to check.
 * @returns {obj is Record<string, any>} True if `obj` is a JSON object, false otherwise.
 *
 * @example
 * isJSONObject({ nested: { boolean: true, array: [1, 2, 3], string: 'test', null: null } }); // true
 * isJSONObject({ regexp: /test/ }); // false
 * isJSONObject(123); // false
 */
declare function isJSONObject(obj: unknown): obj is Record<string, any>;

export { isJSONArray, isJSONObject, isJSONValue };
