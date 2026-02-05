/**
 * Checks if a given value is a valid JSON string.
 *
 * A valid JSON string is one that can be successfully parsed using `JSON.parse()`. According to JSON
 * specifications, valid JSON can represent:
 * - Objects (with string keys and valid JSON values)
 * - Arrays (containing valid JSON values)
 * - Strings
 * - Numbers
 * - Booleans
 * - null
 *
 * String values like `"null"`, `"true"`, `"false"`, and numeric strings (e.g., `"42"`) are considered
 * valid JSON and will return true.
 *
 * This function serves as a type guard in TypeScript, narrowing the type of the argument to `string`.
 *
 * @param {unknown} value The value to check.
 * @returns {boolean} Returns `true` if `value` is a valid JSON string, else `false`.
 *
 * @example
 * isJSON('{"name":"John","age":30}'); // true
 * isJSON('[1,2,3]'); // true
 * isJSON('true'); // true
 * isJSON('invalid json'); // false
 * isJSON({ name: 'John' }); // false (not a string)
 * isJSON(null); // false (not a string)
 */
declare function isJSON(value: unknown): value is string;

export { isJSON };
