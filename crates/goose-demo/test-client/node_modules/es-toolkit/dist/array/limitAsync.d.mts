/**
 * Wraps an async function to limit the number of concurrent executions.
 *
 * This function creates a wrapper around an async callback that ensures at most
 * `concurrency` number of executions can run simultaneously. Additional calls will
 * wait until a slot becomes available.
 *
 * @template F - The type of the async function to wrap.
 * @param {F} callback The async function to wrap with concurrency control.
 * @param {number} concurrency Maximum number of concurrent executions allowed.
 * @returns {F} A wrapped version of the callback with concurrency limiting.
 * @example
 * const limitedFetch = limitAsync(async (url) => {
 *   return await fetch(url);
 * }, 3);
 *
 * // Only 3 fetches will run concurrently
 * const urls = ['url1', 'url2', 'url3', 'url4', 'url5'];
 * await Promise.all(urls.map(url => limitedFetch(url)));
 *
 * @example
 * const processItem = async (item) => {
 *   // Expensive async operation
 *   return await heavyComputation(item);
 * };
 *
 * const limitedProcess = limitAsync(processItem, 2);
 * const items = [1, 2, 3, 4, 5];
 * // At most 2 items will be processed concurrently
 * await Promise.all(items.map(item => limitedProcess(item)));
 */
declare function limitAsync<F extends (...args: any[]) => Promise<any>>(callback: F, concurrency: number): F;

export { limitAsync };
