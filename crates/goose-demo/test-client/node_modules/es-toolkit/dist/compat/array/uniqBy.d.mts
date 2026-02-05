import { ValueIteratee } from '../_internal/ValueIteratee.mjs';

/**
 * Creates a duplicate-free version of an array, using an optional transform function.
 *
 * @template T
 * @param {ArrayLike<T> | null | undefined} array - The array to inspect.
 * @param {ValueIteratee<T>} iteratee - The transform function or property name to get values from.
 * @returns {T[]} Returns the new duplicate-free array.
 *
 * @example
 * uniqBy([2.1, 1.2, 2.3], Math.floor);
 * // => [2.1, 1.2]
 */
declare function uniqBy<T>(array: ArrayLike<T> | null | undefined, iteratee: ValueIteratee<T>): T[];

export { uniqBy };
