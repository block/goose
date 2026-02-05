/**
 * Returns a promise that rejects with a `TimeoutError` after a specified delay.
 *
 * @param {number} ms - The delay duration in milliseconds.
 * @returns {Promise<never>} A promise that rejects with a `TimeoutError` after the specified delay.
 * @throws {TimeoutError} Throws a `TimeoutError` after the specified delay.
 *
 * @example
 * try {
 *   await timeout(1000); // Timeout exception after 1 second
 * } catch (error) {
 *   console.error(error); // Will log 'The operation was timed out'
 * }
 */
declare function timeout(ms: number): Promise<never>;

export { timeout };
