/**
 * Invokes the specified function after a delay of the given number of milliseconds.
 * Any additional arguments are passed to the function when it is invoked.
 *
 * @param {(...args: any[]) => any} func - The function to delay.
 * @param {number} wait - The number of milliseconds to delay the invocation.
 * @param {...any[]} args - The arguments to pass to the function when it is invoked.
 * @returns {number} Returns the timer id.
 * @throws {TypeError} If the first argument is not a function.
 *
 * @example
 * // Example 1: Delayed function execution
 * const timerId = delay(
 *   (greeting, recipient) => {
 *     console.log(`${greeting}, ${recipient}!`);
 *   },
 *   1000,
 *   'Hello',
 *   'Alice'
 * );
 * // => 'Hello, Alice!' will be logged after one second.
 *
 * // Example 2: Clearing the timeout before execution
 * clearTimeout(timerId);
 * // The function will not be executed because the timeout was cleared.
 */
declare function delay(func: (...args: any[]) => any, wait: number, ...args: any[]): number;

export { delay };
