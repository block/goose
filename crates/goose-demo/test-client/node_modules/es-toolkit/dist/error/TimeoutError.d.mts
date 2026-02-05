/**
 * An error class representing an timeout operation.
 * @augments Error
 */
declare class TimeoutError extends Error {
    constructor(message?: string);
}

export { TimeoutError };
