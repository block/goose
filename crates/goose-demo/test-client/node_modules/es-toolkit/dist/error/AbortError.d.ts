/**
 * An error class representing an aborted operation.
 * @augments Error
 */
declare class AbortError extends Error {
    constructor(message?: string);
}

export { AbortError };
