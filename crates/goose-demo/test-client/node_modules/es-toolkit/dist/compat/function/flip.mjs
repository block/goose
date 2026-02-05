function flip(func) {
    return function (...args) {
        return func.apply(this, args.reverse());
    };
}

export { flip };
