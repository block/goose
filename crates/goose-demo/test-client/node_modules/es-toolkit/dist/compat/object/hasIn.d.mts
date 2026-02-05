import { PropertyPath } from '../_internal/PropertyPath.mjs';

/**
 * Checks if a given path exists in an object, **including inherited properties**.
 *
 * You can provide the path as a single property key, an array of property keys,
 * or a string representing a deep path.
 *
 * Unlike `has`, which only checks for own properties, `hasIn` also checks for properties
 * in the prototype chain.
 *
 * If the path is an index and the object is an array or an arguments object, the function will verify
 * if the index is valid and within the bounds of the array or arguments object, even if the array or
 * arguments object is sparse (i.e., not all indexes are defined).
 *
 * @template T
 * @param {T} object - The object to query.
 * @param {PropertyPath} path - The path to check. This can be a single property key,
 *        an array of property keys, or a string representing a deep path.
 * @returns {boolean} Returns `true` if the path exists (own or inherited), `false` otherwise.
 *
 * @example
 *
 * const obj = { a: { b: { c: 3 } } };
 *
 * hasIn(obj, 'a'); // true
 * hasIn(obj, ['a', 'b']); // true
 * hasIn(obj, ['a', 'b', 'c']); // true
 * hasIn(obj, 'a.b.c'); // true
 * hasIn(obj, 'a.b.d'); // false
 * hasIn(obj, ['a', 'b', 'c', 'd']); // false
 *
 * // Example with inherited properties:
 * function Rectangle() {}
 * Rectangle.prototype.area = function() {};
 *
 * const rect = new Rectangle();
 * hasIn(rect, 'area'); // true
 * has(rect, 'area'); // false - has only checks own properties
 */
declare function hasIn<T>(object: T, path: PropertyPath): boolean;

export { hasIn };
