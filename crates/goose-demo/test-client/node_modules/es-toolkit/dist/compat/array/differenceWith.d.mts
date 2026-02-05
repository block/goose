/**
 * Computes the difference between the primary array and another array using a comparator function.
 *
 * @template T1, T2
 * @param {ArrayLike<T1> | null | undefined} array - The primary array to compare elements against.
 * @param {ArrayLike<T2>} values - The array containing elements to compare with the primary array.
 * @param {(a: T1, b: T2) => boolean} comparator - A function to determine if two elements are considered equal.
 * @returns {T1[]} A new array containing the elements from the primary array that do not match any elements in `values` based on the comparator.
 *
 * @example
 * const array = [{ id: 1 }, { id: 2 }, { id: 3 }];
 * const values = [{ id: 2 }];
 * const comparator = (a, b) => a.id === b.id;
 *
 * const result = differenceWith(array, values, comparator);
 * // result will be [{ id: 1 }, { id: 3 }]
 */
declare function differenceWith<T1, T2>(array: ArrayLike<T1> | null | undefined, values: ArrayLike<T2>, comparator: (a: T1, b: T2) => boolean): T1[];
/**
 * Computes the difference between the primary array and two arrays using a comparator function.
 *
 * @template T1, T2, T3
 * @param {ArrayLike<T1> | null | undefined} array - The primary array to compare elements against.
 * @param {ArrayLike<T2>} values1 - The first array containing elements to compare with the primary array.
 * @param {ArrayLike<T3>} values2 - The second array containing elements to compare with the primary array.
 * @param {(a: T1, b: T2 | T3) => boolean} comparator - A function to determine if two elements are considered equal.
 * @returns {T1[]} A new array containing the elements from the primary array that do not match any elements in `values1` or `values2` based on the comparator.
 *
 * @example
 * const array = [{ id: 1 }, { id: 2 }, { id: 3 }];
 * const values1 = [{ id: 2 }];
 * const values2 = [{ id: 3 }];
 * const comparator = (a, b) => a.id === b.id;
 *
 * const result = differenceWith(array, values1, values2, comparator);
 * // result will be [{ id: 1 }]
 */
declare function differenceWith<T1, T2, T3>(array: ArrayLike<T1> | null | undefined, values1: ArrayLike<T2>, values2: ArrayLike<T3>, comparator: (a: T1, b: T2 | T3) => boolean): T1[];
/**
 * Computes the difference between the primary array and multiple arrays using a comparator function.
 *
 * @template T1, T2, T3, T4
 * @param {ArrayLike<T1> | null | undefined} array - The primary array to compare elements against.
 * @param {ArrayLike<T2>} values1 - The first array containing elements to compare with the primary array.
 * @param {ArrayLike<T3>} values2 - The second array containing elements to compare with the primary array.
 * @param {...Array<ArrayLike<T4> | ((a: T1, b: T2 | T3 | T4) => boolean)>} values - Additional arrays and an optional comparator function to determine if two elements are considered equal.
 * @returns {T1[]} A new array containing the elements from the primary array that do not match any elements
 * in `values1`, `values2`, or subsequent arrays. If a comparator function is provided, it will be used to compare elements;
 * otherwise, [SameValueZero](https://tc39.es/ecma262/multipage/abstract-operations.html#sec-samevaluezero) algorithm will be used.
 *
 * @example
 * // Example with comparator function
 * const array = [{ id: 1 }, { id: 2 }, { id: 3 }, { id: 4 }];
 * const values1 = [{ id: 2 }];
 * const values2 = [{ id: 3 }];
 * const values3 = [{ id: 4 }];
 * const comparator = (a, b) => a.id === b.id;
 *
 * const result = differenceWith(array, values1, values2, values3, comparator);
 * // result will be [{ id: 1 }]
 *
 * @example
 * // Example without comparator function (behaves like `difference`)
 * const array = [1, 2, 3, 4];
 * const values1 = [2];
 * const values2 = [3];
 * const values3 = [4];
 *
 * const result = differenceWith(array, values1, values2, values3);
 * // result will be [1]
 */
declare function differenceWith<T1, T2, T3, T4>(array: ArrayLike<T1> | null | undefined, values1: ArrayLike<T2>, values2: ArrayLike<T3>, ...values: Array<ArrayLike<T4> | ((a: T1, b: T2 | T3 | T4) => boolean)>): T1[];
/**
 * Computes the difference between the primary array and one or more arrays without using a comparator function.
 *
 * @template T
 * @param {ArrayLike<T> | null | undefined} array - The primary array to compare elements against.
 * @param {...Array<ArrayLike<T>>} values - One or more arrays containing elements to compare with the primary array.
 * @returns {T[]} A new array containing the elements from the primary array that do not match any elements in the provided arrays.
 *
 * @example
 * const array = [1, 2, 3];
 * const values1 = [2];
 * const values2 = [3];
 *
 * const result = differenceWith(array, values1, values2);
 * // result will be [1]
 */
declare function differenceWith<T>(array: ArrayLike<T> | null | undefined, ...values: Array<ArrayLike<T>>): T[];

export { differenceWith };
