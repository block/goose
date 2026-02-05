/**
 * Creates a function that is restricted to invoking func once. Repeat calls to the function return the value of the first invocation.
 *
 * @template F - The type of the function.
 * @param {F} func - The function to restrict.
 * @returns {F} Returns the new restricted function.
 *
 * @example
 * const initialize = once(createApplication);
 *
 * initialize();
 * initialize();
 * // => `createApplication` is invoked once
 */
declare function once<F extends (...args: any[]) => any>(func: F): F;

export { once };
