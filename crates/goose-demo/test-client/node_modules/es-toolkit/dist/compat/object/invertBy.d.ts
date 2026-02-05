import { ValueIteratee } from '../_internal/ValueIteratee.js';

/**
 * Creates a new object that reverses the keys and values of the given object, similar to the invert.
 * The values of the new object are arrays of keys that correspond to the value returned by the `iteratee` function.
 *
 * @param {Record<string|number, T>} object - The object to iterate over
 * @param {ValueIteratee<T>} [iteratee] - Optional function to transform values into keys
 * @returns {Record<string, string[]>} An object with transformed values as keys and arrays of original keys as values
 *
 * @example
 * const obj = { a: 1, b: 2, c: 1 };
 * invertBy(obj); // => { '1': ['a', 'c'], '2': ['b'] }
 *
 * @example
 * const obj = { a: 1, b: 2, c: 1 };
 * invertBy(obj, value => `group${value}`); // => { 'group1': ['a', 'c'], 'group2': ['b'] }
 */
declare function invertBy<T>(object: Record<string, T> | Record<number, T> | null | undefined, interatee?: ValueIteratee<T>): Record<string, string[]>;
/**
 * Creates a new object that reverses the keys and values of the given object, similar to the invert.
 * The values of the new object are arrays of keys that correspond to the value returned by the `iteratee` function.
 *
 * @param {T} object - The object to iterate over
 * @param {ValueIteratee<T[keyof T]>} [iteratee] - Optional function to transform values into keys
 * @returns {Record<string, string[]>} An object with transformed values as keys and arrays of original keys as values
 *
 * @example
 * const obj = { foo: { id: 1 }, bar: { id: 2 }, baz: { id: 1 } };
 * invertBy(obj, value => String(value.id)); // => { '1': ['foo', 'baz'], '2': ['bar'] }
 */
declare function invertBy<T extends object>(object: T | null | undefined, interatee?: ValueIteratee<T[keyof T]>): Record<string, string[]>;

export { invertBy };
