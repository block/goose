import { PropertyPath } from '../_internal/PropertyPath.js';

/**
 * Creates a function that checks if a given target object matches a specific property value.
 *
 * @template T
 * @template V
 * @param {PropertyPath} path - The property path to check within the target object.
 * @param {T} srcValue - The value to compare against the property value in the target object.
 * @returns {(value: any) => boolean} Returns a function that takes a target object and returns
 *     `true` if the property value at the given path in the target object matches the provided value,
 *     otherwise returns `false`.
 *
 * @example
 * const checkName = matchesProperty('name', 'Alice');
 * console.log(checkName({ name: 'Alice' })); // true
 * console.log(checkName({ name: 'Bob' })); // false
 */
declare function matchesProperty<T>(path: PropertyPath, srcValue: T): (value: any) => boolean;
/**
 * Creates a function that checks if a given target object matches a specific property value.
 *
 * @template T
 * @template V
 * @param {PropertyPath} path - The property path to check within the target object.
 * @param {T} srcValue - The value to compare against the property value in the target object.
 * @returns {(value: V) => boolean} Returns a function that takes a target object and returns
 *     `true` if the property value at the given path in the target object matches the provided value,
 *     otherwise returns `false`.
 *
 * @example
 * const checkNested = matchesProperty(['address', 'city'], 'New York');
 * console.log(checkNested({ address: { city: 'New York' } })); // true
 * console.log(checkNested({ address: { city: 'Los Angeles' } })); // false
 */
declare function matchesProperty<T, V>(path: PropertyPath, srcValue: T): (value: V) => boolean;

export { matchesProperty };
