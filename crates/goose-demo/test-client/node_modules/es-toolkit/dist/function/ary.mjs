function ary(func, n) {
    return function (...args) {
        return func.apply(this, args.slice(0, n));
    };
}

export { ary };
