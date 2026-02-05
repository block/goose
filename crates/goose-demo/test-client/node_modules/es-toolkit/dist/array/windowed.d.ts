/**
 * Options for the windowed function.
 *
 * @interface WindowedOptions
 * @property {boolean} [partialWindows=false] - Whether to include partial windows at the end of the array.
 */
interface WindowedOptions {
    /**
     * Whether to include partial windows at the end of the array.
     *
     * By default, `windowed` only includes full windows in the result,
     * ignoring any leftover elements that can't form a full window.
     *
     * If `partialWindows` is true, the function will also include these smaller, partial windows at the end of the result.
     */
    partialWindows?: boolean;
}
/**
 * Creates an array of sub-arrays (windows) from the input array, each of the specified size.
 * The windows can overlap depending on the step size provided.
 *
 * By default, only full windows are included in the result, and any leftover elements that can't form a full window are ignored.
 *
 * If the `partialWindows` option is set to true in the options object, the function will also include partial windows at the end of the result.
 * Partial windows are smaller sub-arrays created when there aren't enough elements left in the input array to form a full window.
 *
 * @template T
 * @param {readonly T[]} arr - The input array to create windows from.
 * @param {number} size - The size of each window. Must be a positive integer.
 * @param {number} [step=1] - The step size between the start of each window. Must be a positive integer.
 * @param {WindowedOptions} [options={}] - Options object to configure the behavior of the function.
 * @param {boolean} [options.partialWindows=false] - Whether to include partial windows at the end of the array.
 * @returns {T[][]} An array of windows (sub-arrays) created from the input array.
 * @throws {Error} If the size or step is not a positive integer.
 *
 * @example
 * windowed([1, 2, 3, 4], 2);
 * // => [[1, 2], [2, 3], [3, 4]]
 *
 * @example
 * windowed([1, 2, 3, 4, 5, 6], 3, 2);
 * // => [[1, 2, 3], [3, 4, 5]]
 *
 * @example
 * windowed([1, 2, 3, 4, 5, 6], 3, 2, { partialWindows: true });
 * // => [[1, 2, 3], [3, 4, 5], [5, 6]]
 */
declare function windowed<T>(arr: readonly T[], size: number, step?: number, { partialWindows }?: WindowedOptions): T[][];

export { type WindowedOptions, windowed };
