/**
 * Calculates the median of an array of numbers.
 *
 * The median is the middle value of a sorted array.
 * If the array has an odd number of elements, the median is the middle value.
 * If the array has an even number of elements, it returns the average of the two middle values.
 *
 * If the array is empty, this function returns `NaN`.
 *
 * @param {number[]} nums - An array of numbers to calculate the median.
 * @returns {number} The median of all the numbers in the array.
 *
 * @example
 * const arrayWithOddNumberOfElements = [1, 2, 3, 4, 5];
 * const result = median(arrayWithOddNumberOfElements);
 * // result will be 3
 *
 * @example
 * const arrayWithEvenNumberOfElements = [1, 2, 3, 4];
 * const result = median(arrayWithEvenNumberOfElements);
 * // result will be 2.5
 */
declare function median(nums: readonly number[]): number;

export { median };
