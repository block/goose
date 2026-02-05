/**
 * Gathers elements in the same position in an internal array
 * from a grouped array of elements and returns them as a new array.
 *
 * @template T - The type of elements in the nested array.
 * @param {Array<[...T]>} zipped - The nested array to unzip.
 * @returns {Unzip<T>} A new array of unzipped elements.
 *
 * @example
 * const zipped = [['a', true, 1],['b', false, 2]];
 * const result = unzip(zipped);
 * // result will be [['a', 'b'], [true, false], [1, 2]]
 */
declare function unzip<T extends unknown[]>(zipped: ReadonlyArray<[...T]>): Unzip<T>;
type Unzip<K extends unknown[]> = {
    [I in keyof K]: Array<K[I]>;
};

export { unzip };
