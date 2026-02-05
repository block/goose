/**
 * Calculates the average of an array of numbers.
 *
 * If the array is empty, this function returns `NaN`.
 *
 * @param {ArrayLike<any> | null | undefined} nums - An array of numbers to calculate the average.
 * @returns {number} The average of all the numbers in the array.
 *
 * @example
 * const numbers = [1, 2, 3, 4, 5];
 * const result = mean(numbers);
 * // result will be 3
 */
declare function mean(nums: ArrayLike<any> | null | undefined): number;

export { mean };
