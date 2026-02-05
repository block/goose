function rest(func, startIndex = func.length - 1) {
    return function (...args) {
        const rest = args.slice(startIndex);
        const params = args.slice(0, startIndex);
        while (params.length < startIndex) {
            params.push(undefined);
        }
        return func.apply(this, [...params, rest]);
    };
}

export { rest };
