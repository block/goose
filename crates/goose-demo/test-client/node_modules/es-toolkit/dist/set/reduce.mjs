function reduce(set, callback, initialValue) {
    if (initialValue == null && set.size === 0) {
        throw new TypeError('Reduce of empty set with no initial value');
    }
    let accumulator = initialValue;
    for (const value of set) {
        if (accumulator == null) {
            accumulator = value;
        }
        else {
            accumulator = callback(accumulator, value, value, set);
        }
    }
    return accumulator;
}

export { reduce };
