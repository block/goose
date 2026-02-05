/**
 * Removes elements from an array based on a predicate function.
 *
 * This function changes `arr` in place.
 * If you want to remove elements without modifying the original array, use `filter`.
 *
 * @template T
 * @param {T[]} arr - The array to modify.
 * @param {(value: T, index: number, array: T[]) => boolean} shouldRemoveElement - The function invoked per iteration to determine if an element should be removed.
 * @returns {T[]} The modified array with the specified elements removed.
 *
 * @example
 * const numbers = [1, 2, 3, 4, 5];
 * remove(numbers, (value) => value % 2 === 0);
 * console.log(numbers); // [1, 3, 5]
 */
declare function remove<T>(arr: T[], shouldRemoveElement: (value: T, index: number, array: T[]) => boolean): T[];

export { remove };
