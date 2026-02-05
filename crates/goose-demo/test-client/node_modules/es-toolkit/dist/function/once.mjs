function once(func) {
    let called = false;
    let cache;
    return function (...args) {
        if (!called) {
            called = true;
            cache = func(...args);
        }
        return cache;
    };
}

export { once };
