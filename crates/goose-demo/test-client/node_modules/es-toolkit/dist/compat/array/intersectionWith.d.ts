/**
 * Creates an array of unique values that are included in all given arrays, using a comparator function for equality comparisons.
 *
 * @template T, U
 * @param {ArrayLike<T> | null | undefined} array - The array to inspect.
 * @param {ArrayLike<U>} values - The values to compare.
 * @param {(a: T, b: T | U) => boolean} comparator - The comparator invoked per element.
 * @returns {T[]} Returns the new array of intersecting values.
 *
 * @example
 * const objects = [{ 'x': 1, 'y': 2 }, { 'x': 2, 'y': 1 }];
 * const others = [{ 'x': 1, 'y': 1 }, { 'x': 1, 'y': 2 }];
 * intersectionWith(objects, others, (a, b) => a.x === b.x && a.y === b.y);
 * // => [{ 'x': 1, 'y': 2 }]
 */
declare function intersectionWith<T, U>(array: ArrayLike<T> | null | undefined, values: ArrayLike<U>, comparator: (a: T, b: T | U) => boolean): T[];
/**
 * Creates an array of unique values that are included in all given arrays, using a comparator function for equality comparisons.
 *
 * @template T, U, V
 * @param {ArrayLike<T> | null | undefined} array - The array to inspect.
 * @param {ArrayLike<U>} values1 - The first values to compare.
 * @param {ArrayLike<V>} values2 - The second values to compare.
 * @param {(a: T, b: T | U | V) => boolean} comparator - The comparator invoked per element.
 * @returns {T[]} Returns the new array of intersecting values.
 *
 * @example
 * const objects = [{ 'x': 1, 'y': 2 }, { 'x': 2, 'y': 1 }];
 * const others1 = [{ 'x': 1, 'y': 1 }, { 'x': 1, 'y': 2 }];
 * const others2 = [{ 'x': 1, 'y': 2 }, { 'x': 3, 'y': 4 }];
 * intersectionWith(objects, others1, others2, (a, b) => a.x === b.x && a.y === b.y);
 * // => [{ 'x': 1, 'y': 2 }]
 */
declare function intersectionWith<T, U, V>(array: ArrayLike<T> | null | undefined, values1: ArrayLike<U>, values2: ArrayLike<V>, comparator: (a: T, b: T | U | V) => boolean): T[];
/**
 * Creates an array of unique values that are included in all given arrays, using a comparator function for equality comparisons.
 *
 * @template T, U, V, W
 * @param {ArrayLike<T> | null | undefined} array - The array to inspect.
 * @param {ArrayLike<U>} values1 - The first values to compare.
 * @param {ArrayLike<V>} values2 - The second values to compare.
 * @param {...Array<ArrayLike<W> | (a: T, b: T | U | V | W) => boolean>} values - The other arrays to compare, and the comparator to use.
 * @returns {T[]} Returns the new array of intersecting values.
 *
 * @example
 * const objects = [{ 'x': 1, 'y': 2 }, { 'x': 2, 'y': 1 }];
 * const others1 = [{ 'x': 1, 'y': 1 }, { 'x': 1, 'y': 2 }];
 * const others2 = [{ 'x': 1, 'y': 2 }, { 'x': 3, 'y': 4 }];
 * const others3 = [{ 'x': 1, 'y': 2 }, { 'x': 5, 'y': 6 }];
 * intersectionWith(objects, others1, others2, others3, (a, b) => a.x === b.x && a.y === b.y);
 * // => [{ 'x': 1, 'y': 2 }]
 */
declare function intersectionWith<T, U, V, W>(array: ArrayLike<T> | null | undefined, values1: ArrayLike<U>, values2: ArrayLike<V>, ...values: Array<ArrayLike<W> | ((a: T, b: T | U | V | W) => boolean)>): T[];
/**
 * Creates an array of unique values that are included in all given arrays.
 *
 * @template T
 * @param {ArrayLike<T> | null} [array] - The array to inspect.
 * @param {...Array<ArrayLike<T> | (a: T, b: never) => boolean>} values - The values to compare.
 * @returns {T[]} Returns the new array of intersecting values.
 *
 * @example
 * intersectionWith([2, 1], [2, 3]);
 * // => [2]
 */
declare function intersectionWith<T>(array?: ArrayLike<T> | null, ...values: Array<ArrayLike<T> | ((a: T, b: never) => boolean)>): T[];

export { intersectionWith };
