import { MutableList } from '../_internal/MutableList.d.mjs';
import { RejectReadonly } from '../_internal/RejectReadonly.d.mjs';

/**
 * Fills an array with a value.
 * @template T
 * @param {any[] | null | undefined} array - The array to fill
 * @param {T} value - The value to fill array with
 * @returns {T[]} Returns the filled array
 * @example
 * fill([1, 2, 3], 'a')
 * // => ['a', 'a', 'a']
 */
declare function fill<T>(array: any[] | null | undefined, value: T): T[];
/**
 * Fills an array-like object with a value.
 * @template T, AL
 * @param {RejectReadonly<AL> | null | undefined} array - The array-like object to fill
 * @param {T} value - The value to fill array with
 * @returns {ArrayLike<T>} Returns the filled array-like object
 * @example
 * fill({ length: 3 }, 2)
 * // => { 0: 2, 1: 2, 2: 2, length: 3 }
 */
declare function fill<T, AL extends MutableList<any>>(array: RejectReadonly<AL> | null | undefined, value: T): ArrayLike<T>;
/**
 * Fills an array with a value from start up to end.
 * @template T, U
 * @param {U[] | null | undefined} array - The array to fill
 * @param {T} value - The value to fill array with
 * @param {number} [start=0] - The start position
 * @param {number} [end=array.length] - The end position
 * @returns {Array<T | U>} Returns the filled array
 * @example
 * fill([1, 2, 3], 'a', 1, 2)
 * // => [1, 'a', 3]
 */
declare function fill<T, U>(array: U[] | null | undefined, value: T, start?: number, end?: number): Array<T | U>;
/**
 * Fills an array-like object with a value from start up to end.
 * @template T, U
 * @param {U extends readonly any[] ? never : U | null | undefined} array - The array-like object to fill
 * @param {T} value - The value to fill array with
 * @param {number} [start=0] - The start position
 * @param {number} [end=array.length] - The end position
 * @returns {ArrayLike<T | U[0]>} Returns the filled array-like object
 * @example
 * fill({ 0: 1, 1: 2, 2: 3, length: 3 }, 'a', 1, 2)
 * // => { 0: 1, 1: 'a', 2: 3, length: 3 }
 */
declare function fill<T, U extends MutableList<any>>(array: RejectReadonly<U> | null | undefined, value: T, start?: number, end?: number): ArrayLike<T | U[0]>;

export { fill };
