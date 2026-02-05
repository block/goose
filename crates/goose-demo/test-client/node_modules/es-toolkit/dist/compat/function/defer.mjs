function defer(func, ...args) {
    if (typeof func !== 'function') {
        throw new TypeError('Expected a function');
    }
    return setTimeout(func, 1, ...args);
}

export { defer };
