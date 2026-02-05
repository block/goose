import { PropertyPath } from '../_internal/PropertyPath.js';

/**
 * Checks if a given path exists within an object.
 *
 * @template T
 * @template K
 * @param {T} object - The object to query.
 * @param {K} path - The path to check.
 * @returns {object is T & { [P in K]: P extends keyof T ? T[P] : Record<string, unknown> extends T ? T[keyof T] : unknown } & { [key: symbol]: unknown }} Returns a type guard indicating if the path exists in the object.
 *
 * @example
 * const obj = { a: 1, b: { c: 2 } };
 *
 * if (has(obj, 'a')) {
 *   console.log(obj.a); // TypeScript knows obj.a exists
 * }
 *
 * if (has(obj, 'b')) {
 *   console.log(obj.b.c); // TypeScript knows obj.b exists
 * }
 */
declare function has<T, K extends PropertyKey>(object: T, path: K): object is T & {
    [P in K]: P extends keyof T ? T[P] : Record<string, unknown> extends T ? T[keyof T] : unknown;
} & {
    [key: symbol]: unknown;
};
/**
 * Checks if a given path exists within an object.
 *
 * @template T
 * @param {T} object - The object to query.
 * @param {PropertyPath} path - The path to check. This can be a single property key,
 *        an array of property keys, or a string representing a deep path.
 * @returns {boolean} Returns `true` if the path exists in the object, `false` otherwise.
 *
 * @example
 * const obj = { a: { b: { c: 3 } } };
 *
 * has(obj, 'a'); // true
 * has(obj, ['a', 'b']); // true
 * has(obj, ['a', 'b', 'c']); // true
 * has(obj, 'a.b.c'); // true
 * has(obj, 'a.b.d'); // false
 * has(obj, ['a', 'b', 'c', 'd']); // false
 * has([], 0); // false
 * has([1, 2, 3], 2); // true
 * has([1, 2, 3], 5); // false
 */
declare function has<T>(object: T, path: PropertyPath): boolean;

export { has };
