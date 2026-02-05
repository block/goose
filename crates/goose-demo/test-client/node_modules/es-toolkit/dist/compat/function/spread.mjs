function spread(func, argsIndex = 0) {
    argsIndex = Number.parseInt(argsIndex, 10);
    if (Number.isNaN(argsIndex) || argsIndex < 0) {
        argsIndex = 0;
    }
    return function (...args) {
        const array = args[argsIndex];
        const params = args.slice(0, argsIndex);
        if (array) {
            params.push(...array);
        }
        return func.apply(this, params);
    };
}

export { spread };
