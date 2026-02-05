function attempt(func) {
    try {
        return [null, func()];
    }
    catch (error) {
        return [error, null];
    }
}

export { attempt };
