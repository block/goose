/**
 * A counting semaphore for async functions that manages available permits.
 * Semaphores are mainly used to limit the number of concurrent async tasks.
 *
 * Each `acquire` operation takes a permit or waits until one is available.
 * Each `release` operation adds a permit, potentially allowing a waiting task to proceed.
 *
 * The semaphore ensures fairness by maintaining a FIFO (First In, First Out) order for acquirers.
 *
 * @example
 * const sema = new Semaphore(2);
 *
 * async function task() {
 *   await sema.acquire();
 *   try {
 *     // This code can only be executed by two tasks at the same time
 *   } finally {
 *     sema.release();
 *   }
 * }
 *
 * task();
 * task();
 * task(); // This task will wait until one of the previous tasks releases the semaphore.
 */
declare class Semaphore {
    /**
     * The maximum number of concurrent operations allowed.
     * @type {number}
     */
    capacity: number;
    /**
     * The number of available permits.
     * @type {number}
     */
    available: number;
    private deferredTasks;
    /**
     * Creates an instance of Semaphore.
     * @param {number} capacity - The maximum number of concurrent operations allowed.
     *
     * @example
     * const sema = new Semaphore(3); // Allows up to 3 concurrent operations.
     */
    constructor(capacity: number);
    /**
     * Acquires a semaphore, blocking if necessary until one is available.
     * @returns {Promise<void>} A promise that resolves when the semaphore is acquired.
     *
     * @example
     * const sema = new Semaphore(1);
     *
     * async function criticalSection() {
     *   await sema.acquire();
     *   try {
     *     // This code section cannot be executed simultaneously
     *   } finally {
     *     sema.release();
     *   }
     * }
     */
    acquire(): Promise<void>;
    /**
     * Releases a semaphore, allowing one more operation to proceed.
     *
     * @example
     * const sema = new Semaphore(1);
     *
     * async function task() {
     *   await sema.acquire();
     *   try {
     *     // This code can only be executed by two tasks at the same time
     *   } finally {
     *     sema.release(); // Allows another waiting task to proceed.
     *   }
     * }
     */
    release(): void;
}

export { Semaphore };
