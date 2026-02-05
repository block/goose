function attempt(func, ...args) {
    try {
        return func(...args);
    }
    catch (e) {
        return e instanceof Error ? e : new Error(e);
    }
}

export { attempt };
