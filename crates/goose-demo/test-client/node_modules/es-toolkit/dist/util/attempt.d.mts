/**
 * Attempt to execute a function and return the result or error.
 * Returns a tuple where:
 * - On success: [null, Result] - First element is null, second is the result
 * - On error: [Error, null] - First element is the caught error, second is null
 *
 * @template {unknown} T - The type of the result of the function.
 * @template {unknown} E - The type of the error that can be thrown by the function.
 * @param {() => T} func - The function to execute.
 * @returns {[null, T] | [E, null]} A tuple containing either [null, result] or [error, null].
 *
 * @example
 * // Successful execution
 * const [error, result] = attempt(() => 42);
 * // [null, 42]
 *
 * // Failed execution
 * const [error, result] = attempt(() => {
 *   throw new Error('Something went wrong');
 * });
 * // [Error, null]
 *
 * // With type parameter
 * const [error, names] = attempt<string[]>(() => ['Alice', 'Bob']);
 * // [null, ['Alice', 'Bob']]
 *
 * @note
 * Important: This function is not suitable for async functions (functions that return a `Promise`).
 * When passing an async function, it will return `[null, Promise<Result>]`, but won't catch any
 * errors if the Promise is rejected later.
 *
 * For handling async functions, use the `attemptAsync` function instead:
 * ```
 * const [error, data] = await attemptAsync(async () => {
 *   const response = await fetch('https://api.example.com/data');
 *   return response.json();
 * });
 * ```
 */
declare function attempt<T, E>(func: () => T): [null, T] | [E, null];

export { attempt };
