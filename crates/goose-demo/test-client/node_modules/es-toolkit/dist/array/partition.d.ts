/**
 * Splits an array into two groups based on a predicate function.
 *
 * This function takes an array and a predicate function. It returns a tuple of two arrays:
 * the first array contains elements for which the predicate function returns true, and
 * the second array contains elements for which the predicate function returns false.
 *
 * @template T - The type of elements in the array.
 * @template {T} U - The type being filtered for.
 * @param {T[]} arr - The array to partition.
 * @param {(value: T, index: number, array: readonly T[]) => value is U} isInTruthy - A type guard that determines whether an
 * element should be placed in the truthy array. The function is called with each element
 * of the array and its index.
 * @returns {[U[], Exclude<T, U>[]]} A tuple containing two arrays: the first array contains elements for
 * which the predicate returned true, and the second array contains elements for which the
 * predicate returned false.
 *
 * @example
 * const array = [1, 2, 3, 4] as const;
 * const isEven = (x: number): x is 2 | 4 => x % 2 === 0;
 * const [even, odd]: [(2 | 4)[], (2 | 4)[]] = partition(array, isEven);
 * // even will be [2, 4], and odd will be [1, 3]
 */
declare function partition<T, U extends T>(arr: readonly T[], isInTruthy: (value: T, index: number, array: readonly T[]) => value is U): [truthy: U[], falsy: Array<Exclude<T, U>>];
/**
 * Splits an array into two groups based on a predicate function.
 *
 * This function takes an array and a predicate function. It returns a tuple of two arrays:
 * the first array contains elements for which the predicate function returns true, and
 * the second array contains elements for which the predicate function returns false.
 *
 * @template T - The type of elements in the array.
 * @param {T[]} arr - The array to partition.
 * @param {(value: T, index: number) => boolean} isInTruthy - A predicate function that determines
 * whether an element should be placed in the truthy array. The function is called with each
 * element of the array and its index.
 * @returns {[T[], T[]]} A tuple containing two arrays: the first array contains elements for
 * which the predicate returned true, and the second array contains elements for which the
 * predicate returned false.
 *
 * @example
 * const array = [1, 2, 3, 4, 5];
 * const isEven = x => x % 2 === 0;
 * const [even, odd] = partition(array, isEven);
 * // even will be [2, 4], and odd will be [1, 3, 5]
 */
declare function partition<T>(arr: readonly T[], isInTruthy: (value: T, index: number, array: readonly T[]) => boolean): [truthy: T[], falsy: T[]];

export { partition };
