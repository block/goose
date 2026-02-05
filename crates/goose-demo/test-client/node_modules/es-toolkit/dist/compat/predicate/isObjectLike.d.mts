/**
 * Checks if the given value is object-like.
 *
 * A value is object-like if its type is object and it is not null.
 *
 * This function can also serve as a type predicate in TypeScript, narrowing the type of the argument to an object-like value.
 *
 * @param {any} value - The value to test if it is an object-like.
 * @returns {boolean} `true` if the value is an object-like, `false` otherwise.
 *
 * @example
 * const value1 = { a: 1 };
 * const value2 = [1, 2, 3];
 * const value3 = 'abc';
 * const value4 = () => {};
 * const value5 = null;
 *
 * console.log(isObjectLike(value1)); // true
 * console.log(isObjectLike(value2)); // true
 * console.log(isObjectLike(value3)); // false
 * console.log(isObjectLike(value4)); // false
 * console.log(isObjectLike(value5)); // false
 */
declare function isObjectLike(value?: any): boolean;

export { isObjectLike };
