function invariant(condition, message) {
    if (condition) {
        return;
    }
    if (typeof message === 'string') {
        throw new Error(message);
    }
    throw message;
}

export { invariant };
