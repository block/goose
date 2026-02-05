/**
 * Calculates the average of an array of numbers when applying
 * the `getValue` function to each element.
 *
 * If the array is empty, this function returns `NaN`.
 *
 * @template T - The type of elements in the array.
 * @param {T[]} items An array to calculate the average.
 * @param {(element: T) => number} getValue A function that selects a numeric value from each element.
 * @returns {number} The average of all the numbers as determined by the `getValue` function.
 *
 * @example
 * meanBy([{ a: 1 }, { a: 2 }, { a: 3 }], x => x.a); // Returns: 2
 * meanBy([], x => x.a); // Returns: NaN
 */
declare function meanBy<T>(items: readonly T[], getValue: (element: T) => number): number;

export { meanBy };
