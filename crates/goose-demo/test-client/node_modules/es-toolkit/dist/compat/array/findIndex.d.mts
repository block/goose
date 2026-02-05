import { ListIterateeCustom } from '../_internal/ListIterateeCustom.mjs';

/**
 * Finds the index of the first item in an array that has a specific property, where the property name is provided as a PropertyKey.
 *
 * @template T
 * @param {ArrayLike<T> | null | undefined} arr - The array to search through.
 * @param {((item: T, index: number, arr: any) => unknown) | Partial<T> | [keyof T, unknown] | PropertyKey} doesMatch - The criteria to match against the items in the array. This can be a function, a partial object, a key-value pair, or a property name.
 * @param {PropertyKey} propertyToCheck - The property name to check for in the items of the array.
 * @param {number} [fromIndex=0] - The index to start the search from, defaults to 0.
 * @returns {number} - The index of the first item that has the specified property, or `-1` if no match is found.
 *
 * @example
 * // Using a property name
 * const items = [{ id: 1, name: 'Alice' }, { id: 2, name: 'Bob' }];
 * const result = findIndex(items, 'name');
 * console.log(result); // 0
 */
declare function findIndex<T>(arr: ArrayLike<T> | null | undefined, doesMatch?: ListIterateeCustom<T, boolean>, fromIndex?: number): number;

export { findIndex };
