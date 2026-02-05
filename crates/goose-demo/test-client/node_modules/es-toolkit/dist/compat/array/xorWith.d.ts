/**
 * This method is like `xor` except that it accepts `comparator` which is
 * invoked to compare elements of `arrays`. The comparator is invoked
 * with two arguments: (arrVal, othVal).
 *
 * @template T
 * @param {ArrayLike<T> | null | undefined} arrays - The arrays to inspect.
 * @param {(a: T, b: T) => boolean} [comparator] - The comparator invoked per element.
 * @returns {T[]} Returns the new array of values.
 *
 * @example
 * const objects = [{ 'x': 1, 'y': 2 }, { 'x': 2, 'y': 1 }];
 * const others = [{ 'x': 1, 'y': 1 }, { 'x': 1, 'y': 2 }];
 * xorWith(objects, others, isEqual);
 * // => [{ 'x': 2, 'y': 1 }, { 'x': 1, 'y': 1 }]
 */
declare function xorWith<T>(arrays: ArrayLike<T> | null | undefined, comparator?: (a: T, b: T) => boolean): T[];
/**
 * This method is like `xor` except that it accepts `comparator` which is
 * invoked to compare elements of `arrays`. The comparator is invoked
 * with two arguments: (arrVal, othVal).
 *
 * @template T
 * @param {ArrayLike<T> | null | undefined} arrays - The first array to inspect.
 * @param {ArrayLike<T> | null | undefined} arrays2 - The second array to inspect.
 * @param {(a: T, b: T) => boolean} [comparator] - The comparator invoked per element.
 * @returns {T[]} Returns the new array of values.
 *
 * @example
 * xorWith([1, 2], [2, 3], (a, b) => a === b);
 * // => [1, 3]
 */
declare function xorWith<T>(arrays: ArrayLike<T> | null | undefined, arrays2: ArrayLike<T> | null | undefined, comparator?: (a: T, b: T) => boolean): T[];
/**
 * This method is like `xor` except that it accepts `comparator` which is
 * invoked to compare elements of `arrays`. The comparator is invoked
 * with two arguments: (arrVal, othVal).
 *
 * @template T
 * @param {ArrayLike<T> | null | undefined} arrays - The first array to inspect.
 * @param {ArrayLike<T> | null | undefined} arrays2 - The second array to inspect.
 * @param {ArrayLike<T> | null | undefined} arrays3 - The third array to inspect.
 * @param {...Array<(a: T, b: T) => boolean | ArrayLike<T> | null | undefined>} comparator - The comparator invoked per element.
 * @returns {T[]} Returns the new array of values.
 *
 * @example
 * xorWith([1], [2], [3], (a, b) => a === b);
 * // => [1, 2, 3]
 */
declare function xorWith<T>(arrays: ArrayLike<T> | null | undefined, arrays2: ArrayLike<T> | null | undefined, arrays3: ArrayLike<T> | null | undefined, ...comparator: Array<((a: T, b: T) => boolean) | ArrayLike<T> | null | undefined>): T[];

export { xorWith };
