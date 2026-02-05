/**
 * Checks if the given value is an array.
 *
 * This function tests whether the provided value is an array or not.
 * It returns `true` if the value is an array, and `false` otherwise.
 *
 * This function can also serve as a type predicate in TypeScript, narrowing the type of the argument to an array.
 *
 * @param {any} value - The value to test if it is an array.
 * @returns {value is any[]} `true` if the value is an array, `false` otherwise.
 *
 * @example
 * const value1 = [1, 2, 3];
 * const value2 = 'abc';
 * const value3 = () => {};
 *
 * console.log(isArray(value1)); // true
 * console.log(isArray(value2)); // false
 * console.log(isArray(value3)); // false
 */
declare function isArray(value?: any): value is any[];
/**
 * Checks if the given value is an array with generic type support.
 *
 * This function tests whether the provided value is an array or not.
 * It returns `true` if the value is an array, and `false` otherwise.
 *
 * This function can also serve as a type predicate in TypeScript, narrowing the type of the argument to an array.
 *
 * @template T - The type of elements in the array.
 * @param {any} value - The value to test if it is an array.
 * @returns {value is any[]} `true` if the value is an array, `false` otherwise.
 *
 * @example
 * const value1 = [1, 2, 3];
 * const value2 = 'abc';
 * const value3 = () => {};
 *
 * console.log(isArray<number>(value1)); // true
 * console.log(isArray<string>(value2)); // false
 * console.log(isArray<Function>(value3)); // false
 */
declare function isArray<T>(value?: any): value is any[];

export { isArray };
