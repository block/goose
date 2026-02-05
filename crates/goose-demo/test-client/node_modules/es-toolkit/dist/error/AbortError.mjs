class AbortError extends Error {
    constructor(message = 'The operation was aborted') {
        super(message);
        this.name = 'AbortError';
    }
}

export { AbortError };
