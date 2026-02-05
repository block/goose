type Falsey = false | null | 0 | 0n | '' | undefined;
/**
 * Removes falsey values (false, null, 0, 0n, '', undefined, NaN) from an array.
 *
 * @template T - The type of elements in the array.
 * @param {ArrayLike<T | Falsey> | null | undefined} arr - The input array to remove falsey values.
 * @returns {Array<Exclude<T, false | null | 0 | 0n | '' | undefined>>} - A new array with all falsey values removed.
 *
 * @example
 * compact([0, 0n, 1, false, 2, '', 3, null, undefined, 4, NaN, 5]);
 * Returns: [1, 2, 3, 4, 5]
 */
declare function compact<T>(arr: ArrayLike<T | Falsey> | null | undefined): T[];

export { compact };
