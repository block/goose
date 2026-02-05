class TimeoutError extends Error {
    constructor(message = 'The operation was timed out') {
        super(message);
        this.name = 'TimeoutError';
    }
}

export { TimeoutError };
