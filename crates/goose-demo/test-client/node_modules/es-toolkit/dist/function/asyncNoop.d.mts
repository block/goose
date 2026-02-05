/**
 * An asynchronous no-operation function that does nothing.
 * This can be used as a placeholder or default function.
 *
 * @example
 * asyncNoop(); // Does nothing
 *
 * @returns {Promise<void>} This function returns a Promise that resolves to undefined.
 */
declare function asyncNoop(): Promise<void>;

export { asyncNoop };
