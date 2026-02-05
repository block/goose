/**
 * Checks if `value` is array-like. This overload is for compatibility with lodash type checking.
 *
 * @param {T} t The value to check.
 * @returns {boolean} Returns `true` if `value` is array-like, else `false`.
 */
declare function isArrayLike<T extends {
    __lodashAnyHack: any;
}>(t: T): boolean;
/**
 * Checks if `value` is array-like. Functions, null, and undefined are never array-like.
 *
 * @param {((...args: any[]) => any) | null | undefined} value The value to check.
 * @returns {value is never} Returns `false` for functions, null, and undefined.
 */
declare function isArrayLike(value: ((...args: any[]) => any) | null | undefined): value is never;
/**
 * Checks if `value` is array-like.
 *
 * @param {any} value The value to check.
 * @returns {value is { length: number }} Returns `true` if `value` is array-like, else `false`.
 */
declare function isArrayLike(value: any): value is {
    length: number;
};

export { isArrayLike };
