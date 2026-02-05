/**
 * Gathers elements in the same position in an internal array
 * from a grouped array of elements and returns them as a new array.
 *
 * @template T - The type of elements in the nested array.
 * @param {T[][] | ArrayLike<ArrayLike<T>> | null | undefined} array - The nested array to unzip.
 * @returns {T[][]} A new array of unzipped elements.
 *
 * @example
 * const zipped = [['a', true, 1],['b', false, 2]];
 * const result = unzip(zipped);
 * // result will be [['a', 'b'], [true, false], [1, 2]]
 */
declare function unzip<T>(array: T[][] | ArrayLike<ArrayLike<T>> | null | undefined): T[][];

export { unzip };
