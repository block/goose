/**
 * Attempt to execute an async function and return the result or error.
 * Returns a Promise that resolves to a tuple where:
 * - On success: [null, Result] - First element is null, second is the result
 * - On error: [Error, null] - First element is the caught error, second is null
 *
 * @template {unknown} T - The type of the result of the async function.
 * @template {unknown} E - The type of the error that can be thrown by the async function.
 * @param {() => Promise<T>} func - The async function to execute.
 * @returns {Promise<[null, T] | [E, null]>} A Promise that resolves to a tuple containing either [null, result] or [error, null].
 *
 * @example
 * // Successful execution
 * const [error, data] = await attemptAsync(async () => {
 *   const response = await fetch('https://api.example.com/data');
 *   return response.json();
 * });
 * // If successful: [null, { ... data ... }]
 *
 * // Failed execution
 * const [error, data] = await attemptAsync(async () => {
 *   throw new Error('Network error');
 * });
 * // [Error, null]
 *
 * // With type parameter
 * const [error, users] = await attemptAsync<User[]>(async () => {
 *   const response = await fetch('https://api.example.com/users');
 *   return response.json();
 * });
 * // users is typed as User[]
 */
declare function attemptAsync<T, E>(func: () => Promise<T>): Promise<[null, T] | [E, null]>;

export { attemptAsync };
