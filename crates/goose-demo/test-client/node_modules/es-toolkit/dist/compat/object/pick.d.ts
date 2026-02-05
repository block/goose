import { Many } from '../_internal/Many.js';
import { PropertyPath } from '../_internal/PropertyPath.js';

/**
 * Creates a new object composed of the picked object properties.
 *
 * @template T - The type of object.
 * @template U - The type of keys to pick.
 * @param {T} object - The object to pick keys from.
 * @param {...Array<Many<U>>} props - An array of keys to be picked from the object.
 * @returns {Pick<T, U>} A new object with the specified keys picked.
 *
 * @example
 * const obj = { a: 1, b: 2, c: 3 };
 * const result = pick(obj, ['a', 'c']);
 * // result will be { a: 1, c: 3 }
 */
declare function pick<T extends object, U extends keyof T>(object: T, ...props: Array<Many<U>>): Pick<T, U>;
/**
 * Creates a new object composed of the picked object properties.
 *
 * @template T - The type of object.
 * @param {T | null | undefined} object - The object to pick keys from.
 * @param {...Array<Many<PropertyPath>>} props - An array of keys to be picked from the object.
 * @returns {Partial<T>} A new object with the specified keys picked.
 *
 * @example
 * const obj = { a: 1, b: 2, c: 3 };
 * const result = pick(obj, ['a', 'c']);
 * // result will be { a: 1, c: 3 }
 */
declare function pick<T>(object: T | null | undefined, ...props: Array<Many<PropertyPath>>): Partial<T>;

export { pick };
