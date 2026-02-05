/**
 * Returns a sample element array of a specified size from a collection.
 *
 * @template T
 * @param {Record<string, T> | Record<number, T> | null | undefined} collection - The collection to sample from.
 * @param {number} [n] - The size of sample.
 * @returns {T[]} A new array with sample size applied.
 *
 * @example
 * sampleSize([1, 2, 3], 2);
 * // => [2, 3] (example, actual result will vary)
 */
declare function sampleSize<T>(collection: Record<string, T> | Record<number, T> | null | undefined, n?: number): T[];
/**
 * Returns a sample element array of a specified size from an object.
 *
 * @template T
 * @param {T | null | undefined} collection - The object to sample from.
 * @param {number} [n] - The size of sample.
 * @returns {Array<T[keyof T]>} A new array with sample size applied.
 *
 * @example
 * sampleSize({ a: 1, b: 2, c: 3 }, 2);
 * // => [2, 3] (example, actual result will vary)
 */
declare function sampleSize<T extends object>(collection: T | null | undefined, n?: number): Array<T[keyof T]>;

export { sampleSize };
