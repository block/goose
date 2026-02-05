/**
 * Attempts to execute a function with the provided arguments.
 * If the function throws an error, it catches the error and returns it.
 * If the caught error is not an instance of Error, it wraps it in a new Error.
 *
 * @param {(...args: any[]) => R} func - The function to be executed.
 * @param {...any[]} args - The arguments to pass to the function.
 * @returns {R | Error} The return value of the function if successful, or an Error if an exception is thrown.
 *
 * @template R - The type of the function return value.
 *
 * @example
 * // Example 1: Successful execution
 * const result = attempt((x, y) => x + y, 2, 3);
 * console.log(result); // Output: 5
 *
 * @example
 * // Example 2: Function throws an error
 * const errorResult = attempt(() => {
 *   throw new Error("Something went wrong");
 * });
 * console.log(errorResult); // Output: Error: Something went wrong
 *
 * @example
 * // Example 3: Non-Error thrown
 * const nonErrorResult = attempt(() => {
 *   throw "This is a string error";
 * });
 * console.log(nonErrorResult); // Output: Error: This is a string error
 */
declare function attempt<R>(func: (...args: any[]) => R, ...args: any[]): R | Error;

export { attempt };
